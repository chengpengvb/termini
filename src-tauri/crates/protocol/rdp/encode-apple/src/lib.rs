#![cfg(any(target_os = "macos", target_os = "ios"))]

use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_media::{kCMSampleAttachmentKey_NotSync, kCMTimeInvalid, kCMVideoCodecType_H264, CMSampleBuffer, CMTime, CMVideoFormatDescriptionGetH264ParameterSetAtIndex};
use objc2_core_video::{
    kCVPixelBufferHeightKey, kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferOpenGLCompatibilityKey, kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey,
    kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, CVPixelBuffer, CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags,
    CVPixelBufferPool, CVPixelBufferUnlockBaseAddress,
};
use objc2_video_toolbox::{
    kVTCompressionPropertyKey_AllowFrameReordering, kVTCompressionPropertyKey_AverageBitRate, kVTCompressionPropertyKey_DataRateLimits, kVTCompressionPropertyKey_ExpectedFrameRate,
    kVTCompressionPropertyKey_MaxKeyFrameInterval, kVTCompressionPropertyKey_MaxKeyFrameIntervalDuration, kVTCompressionPropertyKey_PrioritizeEncodingSpeedOverQuality,
    kVTCompressionPropertyKey_ProfileLevel, kVTCompressionPropertyKey_RealTime, kVTEncodeFrameOptionKey_ForceKeyFrame, kVTProfileLevel_H264_ConstrainedHigh_AutoLevel,
    kVTProfileLevel_H264_Main_AutoLevel, kVTVideoEncoderSpecification_EnableLowLatencyRateControl, kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder, VTCompressionSession,
    VTEncodeInfoFlags, VTSessionSetProperty,
};
use std::time::Duration;
use std::ffi::{c_int, c_void};
use std::ptr::{null_mut, NonNull};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use yuv::{rgba_to_yuv_nv12, BufferStoreMut, YuvBiPlanarImageMut, YuvConversionMode, YuvRange, YuvStandardMatrix};

type OSStatus = i32;

const GOP_FRAMES: i64 = 7200;
const GOP_SECONDS: i64 = 240;
const EXPECTED_FPS: i64 = 60;
const TIMESCALE_MS: i32 = 1000;
const BITRATE_BPS: i64 = 28_000_000;
const ANNEXB_START: [u8; 4] = [0, 0, 0, 1];
const BITRATE_WINDOW_BYTES: i64 = 5_250_000;
struct CallbackSlot {
    sender: Sender<Result<Vec<u8>, String>>,
}

pub struct HwEncoder {
    session: CFRetained<VTCompressionSession>,
    slot: *mut CallbackSlot,
    receiver: Receiver<Result<Vec<u8>, String>>,
    width: u16,
    height: u16,
    name: &'static str,
}

fn cf_bool(value: bool) -> &'static CFType {
    CFBoolean::new(value)
}

unsafe fn process_sample(status: OSStatus, sample: *mut CMSampleBuffer) -> Result<Vec<u8>, String> {
    if status != 0 {
        return Err(format!("VideoToolbox encode failed: {status}"));
    }
    let Some(sample) = sample.as_ref() else {
        return Err("VideoToolbox produced no sample".to_string());
    };
    let keyframe = match sample.sample_attachments_array(false) {
        None => true,
        Some(array) => {
            if array.count() == 0 {
                true
            } else {
                let entry = array.value_at_index(0) as *const CFDictionary;
                let Some(entry) = entry.as_ref() else {
                    return Err("VideoToolbox sample attachments are empty".to_string());
                };
                let not_sync = entry.value(kCMSampleAttachmentKey_NotSync as *const CFString as *const c_void);
                not_sync.is_null()
            }
        }
    };
    let mut annexb: Vec<u8> = Vec::new();
    let mut nal_length_size: c_int = 4;
    if keyframe {
        let Some(description) = sample.format_description() else {
            return Err("VideoToolbox sample has no format description".to_string());
        };
        let mut count = 0usize;
        let status = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(&description, 0, null_mut(), null_mut(), &mut count, &mut nal_length_size);
        if status != 0 {
            return Err(format!("failed to read parameter set count: {status}"));
        }
        for index in 0..count {
            let mut pointer: *const u8 = null_mut();
            let mut size = 0usize;
            let status = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(&description, index, &mut pointer, &mut size, null_mut(), null_mut());
            if status != 0 {
                return Err(format!("failed to read parameter set {index}: {status}"));
            }
            annexb.extend_from_slice(&ANNEXB_START);
            annexb.extend_from_slice(std::slice::from_raw_parts(pointer, size));
        }
    }
    let Some(block) = sample.data_buffer() else {
        return Err("VideoToolbox sample has no data".to_string());
    };
    let length = block.data_length();
    if length == 0 {
        return Err("VideoToolbox output data is empty".to_string());
    }
    let mut avcc = vec![0u8; length];
    let destination = NonNull::new(avcc.as_mut_ptr() as *mut c_void).ok_or_else(|| "output buffer pointer is null".to_string())?;
    let status = block.copy_data_bytes(0, length, destination);
    if status != 0 {
        return Err(format!("failed to copy encoded data: {status}"));
    }
    let nal_length_size = nal_length_size as usize;
    let mut offset = 0usize;
    while offset + nal_length_size <= avcc.len() {
        let mut size = 0usize;
        for byte in &avcc[offset..offset + nal_length_size] {
            size = (size << 8) | *byte as usize;
        }
        let start = offset + nal_length_size;
        let end = start + size;
        if end > avcc.len() {
            return Err(format!("AVCC NAL out of bounds: end {end} len {}", avcc.len()));
        }
        annexb.extend_from_slice(&ANNEXB_START);
        annexb.extend_from_slice(&avcc[start..end]);
        offset = end;
    }
    Ok(annexb)
}

unsafe extern "C-unwind" fn output_callback(refcon: *mut c_void, _frame_refcon: *mut c_void, status: OSStatus, flags: VTEncodeInfoFlags, sample: *mut CMSampleBuffer) {
    if flags.contains(VTEncodeInfoFlags::FrameDropped) {
        return;
    }
    let slot = &*(refcon as *const CallbackSlot);
    let _ = slot.sender.send(process_sample(status, sample));
}

impl HwEncoder {
    pub fn new(width: u16, height: u16) -> Result<Self, String> {
        let attempts: [(bool, bool, &'static str); 3] =
            [(true, true, "VideoToolbox H264 ConstrainedHigh low-latency"), (false, true, "VideoToolbox H264 ConstrainedHigh"), (false, false, "VideoToolbox H264 Main")];
        let mut last_error = String::from("failed to create hardware encoder session");
        for (low_latency, constrained_high, name) in attempts {
            let (sender, receiver) = channel();
            let slot = Box::into_raw(Box::new(CallbackSlot { sender }));
            match unsafe { Self::create_session(width, height, slot, low_latency, constrained_high) } {
                Ok(session) => return Ok(Self { session, slot, receiver, width, height, name }),
                Err(error) => {
                    drop(unsafe { Box::from_raw(slot) });
                    last_error = error;
                }
            }
        }
        Err(last_error)
    }

    unsafe fn create_session(width: u16, height: u16, slot: *mut CallbackSlot, low_latency: bool, constrained_high: bool) -> Result<CFRetained<VTCompressionSession>, String> {
        unsafe {
            let require_hw = cf_bool(true);
            let encoder_specification: CFRetained<CFDictionary<CFString, CFType>> = if low_latency {
                CFDictionary::from_slices(
                    &[kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder, kVTVideoEncoderSpecification_EnableLowLatencyRateControl],
                    &[require_hw, cf_bool(true)],
                )
            } else {
                CFDictionary::from_slices(&[kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder], &[require_hw])
            };

            let format = CFNumber::new_i64(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange as i64);
            let buffer_width = CFNumber::new_i64(width as i64);
            let buffer_height = CFNumber::new_i64(height as i64);
            let iosurface: CFRetained<CFDictionary<CFString, CFType>> = CFDictionary::from_slices(&[], &[]);
            let source_attributes: CFRetained<CFDictionary<CFString, CFType>> = CFDictionary::from_slices(
                &[kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey, kCVPixelBufferHeightKey, kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferOpenGLCompatibilityKey],
                &[&format, &buffer_width, &buffer_height, &iosurface, cf_bool(true)],
            );

            let mut session_out: *mut VTCompressionSession = null_mut();
            let status = VTCompressionSession::create(
                None,
                width as i32,
                height as i32,
                kCMVideoCodecType_H264,
                Some(encoder_specification.as_opaque()),
                Some(source_attributes.as_opaque()),
                None,
                Some(output_callback),
                slot as *mut c_void,
                NonNull::from(&mut session_out),
            );
            if status != 0 {
                return Err(format!("failed to create hardware encoder session: {status}"));
            }
            let session = CFRetained::from_raw(NonNull::new(session_out).ok_or_else(|| "hardware encoder session is null".to_string())?);
            let profile: &CFString = if constrained_high { kVTProfileLevel_H264_ConstrainedHigh_AutoLevel } else { kVTProfileLevel_H264_Main_AutoLevel };

            let bitrate = CFNumber::new_i64(BITRATE_BPS);
            let gop = CFNumber::new_i64(GOP_FRAMES);
            let gop_seconds = CFNumber::new_i64(GOP_SECONDS);
            let fps = CFNumber::new_i64(EXPECTED_FPS);
            let required: [(&CFString, &CFType); 6] = [
                (kVTCompressionPropertyKey_RealTime, cf_bool(true)),
                (kVTCompressionPropertyKey_AllowFrameReordering, cf_bool(false)),
                (kVTCompressionPropertyKey_ProfileLevel, profile),
                (kVTCompressionPropertyKey_AverageBitRate, &bitrate),
                (kVTCompressionPropertyKey_MaxKeyFrameInterval, &gop),
                (kVTCompressionPropertyKey_ExpectedFrameRate, &fps),
            ];
            for (key, value) in required {
                let status = VTSessionSetProperty(&session, key, Some(value));
                if status != 0 {
                    return Err(format!("failed to set encoder property {key}: {status}"));
                }
            }
            let _ = VTSessionSetProperty(&session, kVTCompressionPropertyKey_PrioritizeEncodingSpeedOverQuality, Some(cf_bool(true)));
            let _ = VTSessionSetProperty(&session, kVTCompressionPropertyKey_MaxKeyFrameIntervalDuration, Some(&gop_seconds));
            let bytes_limit = CFNumber::new_i64(BITRATE_WINDOW_BYTES);
            let window = CFNumber::new_i64(1);
            let limits = CFArray::from_retained_objects(&[bytes_limit, window]);
            let _ = VTSessionSetProperty(&session, kVTCompressionPropertyKey_DataRateLimits, Some(&limits));
            let status = session.prepare_to_encode_frames();
            if status != 0 {
                return Err(format!("failed to prepare encoder session: {status}"));
            }
            Ok(session)
        }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn set_bitrate(&mut self, bitrate: u32) -> Result<(), String> {
        let value = CFNumber::new_i64(i64::from(bitrate));
        let status = unsafe { VTSessionSetProperty(&self.session, kVTCompressionPropertyKey_AverageBitRate, Some(&value)) };
        if status != 0 {
            return Err(format!("failed to set encoder bitrate: {status}"));
        }
        Ok(())
    }

    pub fn submit(&mut self, rgba: &[u8], timestamp_ms: u64, force_idr: bool) -> Result<(), String> {
        let expected = self.width as usize * self.height as usize * 4;
        if rgba.len() != expected {
            return Err(format!("H264 encoder pixel length mismatch: need {} have {}", expected, rgba.len()));
        }
        unsafe {
            let pixel_buffer = self.build_pixel_buffer_from_rgba(rgba)?;
            self.encode_pixel_buffer(&pixel_buffer, timestamp_ms, force_idr)
        }
    }

    pub fn submit_nv12(&mut self, y: &[u8], uv: &[u8], timestamp_ms: u64, force_idr: bool) -> Result<(), String> {
        let width = self.width as usize;
        let height = self.height as usize;
        if y.len() != width * height || uv.len() != width * height.div_ceil(2) {
            return Err(format!("H264 encoder NV12 length mismatch: y={} uv={} size={}x{}", y.len(), uv.len(), self.width, self.height));
        }
        unsafe {
            let pixel_buffer = self.build_pixel_buffer_from_nv12(y, uv)?;
            self.encode_pixel_buffer(&pixel_buffer, timestamp_ms, force_idr)
        }
    }

    unsafe fn encode_pixel_buffer(&self, pixel_buffer: &CVPixelBuffer, timestamp_ms: u64, force_idr: bool) -> Result<(), String> {
        let frame_properties: Option<CFRetained<CFDictionary<CFString, CFType>>> =
            if force_idr { Some(CFDictionary::from_slices(&[kVTEncodeFrameOptionKey_ForceKeyFrame], &[cf_bool(true)])) } else { None };
        let timestamp = CMTime::new(timestamp_ms as i64, TIMESCALE_MS);
        let status = self.session.encode_frame(pixel_buffer, timestamp, kCMTimeInvalid, frame_properties.as_deref().map(|properties| properties.as_opaque()), null_mut(), null_mut());
        if status != 0 {
            return Err(format!("failed to submit encoder frame: {status}"));
        }
        Ok(())
    }

    pub fn take_output(&mut self, wait: Duration) -> Result<Option<Vec<u8>>, String> {
        match self.receiver.recv_timeout(wait) {
            Ok(Ok(bytes)) if bytes.is_empty() => Err("H264 encoder produced empty output".to_string()),
            Ok(Ok(bytes)) => Ok(Some(bytes)),
            Ok(Err(error)) => Err(error),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err("encoder callback channel closed".to_string()),
        }
    }

    unsafe fn lock_pixel_buffer(&self) -> Result<CFRetained<CVPixelBuffer>, String> {
        let pool = self.session.pixel_buffer_pool().ok_or_else(|| "encoder session has no pixel buffer pool".to_string())?;
        let mut buffer_out: *mut CVPixelBuffer = null_mut();
        let status = CVPixelBufferPool::create_pixel_buffer(None, &pool, NonNull::from(&mut buffer_out));
        if status != 0 {
            return Err(format!("failed to create pixel buffer: {status}"));
        }
        let pixel_buffer = CFRetained::from_raw(NonNull::new(buffer_out).ok_or_else(|| "pixel buffer is null".to_string())?);
        let status = CVPixelBufferLockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
        if status != 0 {
            return Err(format!("failed to lock pixel buffer: {status}"));
        }
        Ok(pixel_buffer)
    }

    unsafe fn build_pixel_buffer_from_rgba(&self, rgba: &[u8]) -> Result<CFRetained<CVPixelBuffer>, String> {
        let pixel_buffer = self.lock_pixel_buffer()?;
        let width = self.width as usize;
        let height = self.height as usize;
        let y_target = CVPixelBufferGetBaseAddressOfPlane(&pixel_buffer, 0) as *mut u8;
        let y_stride = CVPixelBufferGetBytesPerRowOfPlane(&pixel_buffer, 0);
        let uv_target = CVPixelBufferGetBaseAddressOfPlane(&pixel_buffer, 1) as *mut u8;
        let uv_stride = CVPixelBufferGetBytesPerRowOfPlane(&pixel_buffer, 1);
        if y_target.is_null() || uv_target.is_null() || y_stride < width || uv_stride < width {
            CVPixelBufferUnlockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
            return Err(format!("pixel buffer plane invalid: y={y_target:?} uv={uv_target:?} y_stride={y_stride} uv_stride={uv_stride}"));
        }
        let y_plane = std::slice::from_raw_parts_mut(y_target, y_stride * height);
        let uv_plane = std::slice::from_raw_parts_mut(uv_target, uv_stride * height.div_ceil(2));
        let mut nv12 = YuvBiPlanarImageMut {
            y_plane: BufferStoreMut::Borrowed(y_plane),
            y_stride: y_stride as u32,
            uv_plane: BufferStoreMut::Borrowed(uv_plane),
            uv_stride: uv_stride as u32,
            width: self.width as u32,
            height: self.height as u32,
        };
        let matrix = if self.width >= 1280 { YuvStandardMatrix::Bt709 } else { YuvStandardMatrix::Bt601 };
        let converted = rgba_to_yuv_nv12(&mut nv12, rgba, self.width as u32 * 4, YuvRange::Limited, matrix, YuvConversionMode::Balanced);
        let status = CVPixelBufferUnlockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
        converted.map_err(|error| format!("RGBA to NV12 conversion failed: {error}"))?;
        if status != 0 {
            return Err(format!("failed to unlock pixel buffer: {status}"));
        }
        Ok(pixel_buffer)
    }

    unsafe fn build_pixel_buffer_from_nv12(&self, y: &[u8], uv: &[u8]) -> Result<CFRetained<CVPixelBuffer>, String> {
        let pixel_buffer = self.lock_pixel_buffer()?;
        let width = self.width as usize;
        let height = self.height as usize;
        let y_target = CVPixelBufferGetBaseAddressOfPlane(&pixel_buffer, 0) as *mut u8;
        let y_stride = CVPixelBufferGetBytesPerRowOfPlane(&pixel_buffer, 0);
        let uv_target = CVPixelBufferGetBaseAddressOfPlane(&pixel_buffer, 1) as *mut u8;
        let uv_stride = CVPixelBufferGetBytesPerRowOfPlane(&pixel_buffer, 1);
        if y_target.is_null() || uv_target.is_null() || y_stride < width || uv_stride < width {
            CVPixelBufferUnlockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
            return Err(format!("pixel buffer plane invalid: y={y_target:?} uv={uv_target:?} y_stride={y_stride} uv_stride={uv_stride}"));
        }
        for row in 0..height {
            let destination = y_target.add(row * y_stride);
            let origin = row * width;
            std::ptr::copy_nonoverlapping(y.as_ptr().add(origin), destination, width);
        }
        let uv_rows = height.div_ceil(2);
        for row in 0..uv_rows {
            let destination = uv_target.add(row * uv_stride);
            let origin = row * width;
            std::ptr::copy_nonoverlapping(uv.as_ptr().add(origin), destination, width);
        }
        let status = CVPixelBufferUnlockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
        if status != 0 {
            return Err(format!("failed to unlock pixel buffer: {status}"));
        }
        Ok(pixel_buffer)
    }
}

impl Drop for HwEncoder {
    fn drop(&mut self) {
        unsafe {
            self.session.invalidate();
            drop(Box::from_raw(self.slot));
        }
    }
}

unsafe impl Send for HwEncoder {}
