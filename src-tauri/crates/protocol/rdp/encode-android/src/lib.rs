#![cfg(target_os = "android")]

use std::time::Duration;
use ndk::media::media_format::MediaFormat;
use yuv::{rgba_to_yuv_nv12, YuvBiPlanarImageMut, YuvChromaSubsampling, YuvConversionMode, YuvRange, YuvStandardMatrix};
use ndk::media::media_codec::{DequeuedInputBufferResult, DequeuedOutputBufferInfoResult, MediaCodec, MediaCodecDirection};

const GOP_SECONDS: i32 = 240;
const EXPECTED_FPS: i32 = 60;
const BITRATE_BPS: i32 = 28_000_000;
const MIME_TYPE: &str = "video/avc";
const COLOR_FORMAT_NV12: i32 = 21;
const BITRATE_MODE_VBR: i32 = 1;
const BITRATE_MODE_CBR: i32 = 2;
const BUFFER_FLAG_KEY_FRAME: u32 = 1;
const BUFFER_FLAG_CODEC_CONFIG: u32 = 2;
const INPUT_WAIT: Duration = Duration::from_millis(100);

pub struct HwEncoder {
    codec: MediaCodec,
    parameter_sets: Vec<u8>,
    width: u16,
    height: u16,
    name: &'static str,
}

impl HwEncoder {
    pub fn new(width: u16, height: u16) -> Result<Self, String> {
        let attempts: [(i32, &'static str); 2] = [(BITRATE_MODE_CBR, "MediaCodec H264 CBR"), (BITRATE_MODE_VBR, "MediaCodec H264 VBR")];
        let mut last_error = String::from("failed to create hardware encoder codec");
        for (bitrate_mode, name) in attempts {
            match Self::create_codec(width, height, bitrate_mode) {
                Ok(codec) => return Ok(Self { codec, parameter_sets: Vec::new(), width, height, name }),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    fn create_codec(width: u16, height: u16, bitrate_mode: i32) -> Result<MediaCodec, String> {
        let codec = MediaCodec::from_encoder_type(MIME_TYPE).ok_or_else(|| format!("failed to create {MIME_TYPE} encoder codec"))?;
        let mut format = MediaFormat::new();
        format.set_str("mime", MIME_TYPE);
        format.set_i32("width", i32::from(width));
        format.set_i32("height", i32::from(height));
        format.set_i32("color-format", COLOR_FORMAT_NV12);
        format.set_i32("bitrate", BITRATE_BPS);
        format.set_i32("frame-rate", EXPECTED_FPS);
        format.set_i32("i-frame-interval", GOP_SECONDS);
        format.set_i32("bitrate-mode", bitrate_mode);
        format.set_i32("latency", 1);
        format.set_i32("priority", 0);
        codec.configure(&format, None, MediaCodecDirection::Encoder).map_err(|error| format!("failed to configure encoder codec: {error}"))?;
        codec.start().map_err(|error| format!("failed to start encoder codec: {error}"))?;
        Ok(codec)
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn set_bitrate(&mut self, bitrate: u32) -> Result<(), String> {
        let value = i32::try_from(bitrate).map_err(|error| format!("encoder bitrate out of range: {bitrate} ({error})"))?;
        let mut parameters = MediaFormat::new();
        parameters.set_i32("video-bitrate", value);
        self.codec.set_parameters(parameters).map_err(|error| format!("failed to set encoder bitrate: {error}"))
    }

    pub fn submit(&mut self, rgba: &[u8], timestamp_ms: u64, force_idr: bool) -> Result<(), String> {
        let expected = self.width as usize * self.height as usize * 4;
        if rgba.len() != expected {
            return Err(format!("H264 encoder pixel length mismatch: need {} have {}", expected, rgba.len()));
        }
        let mut nv12 = YuvBiPlanarImageMut::<u8>::alloc(self.width as u32, self.height as u32, YuvChromaSubsampling::Yuv420);
        let matrix = if self.width >= 1280 { YuvStandardMatrix::Bt709 } else { YuvStandardMatrix::Bt601 };
        rgba_to_yuv_nv12(&mut nv12, rgba, self.width as u32 * 4, YuvRange::Limited, matrix, YuvConversionMode::Balanced).map_err(|error| format!("RGBA to NV12 conversion failed: {error}"))?;
        self.queue_nv12(nv12.y_plane.borrow(), nv12.uv_plane.borrow(), timestamp_ms, force_idr)
    }

    pub fn submit_nv12(&mut self, y: &[u8], uv: &[u8], timestamp_ms: u64, force_idr: bool) -> Result<(), String> {
        let width = self.width as usize;
        let height = self.height as usize;
        if y.len() != width * height || uv.len() != width * height.div_ceil(2) {
            return Err(format!("H264 encoder NV12 length mismatch: y={} uv={} size={}x{}", y.len(), uv.len(), self.width, self.height));
        }
        self.queue_nv12(y, uv, timestamp_ms, force_idr)
    }

    fn queue_nv12(&self, y: &[u8], uv: &[u8], timestamp_ms: u64, force_idr: bool) -> Result<(), String> {
        if force_idr {
            let mut parameters = MediaFormat::new();
            parameters.set_i32("request-sync", 0);
            self.codec.set_parameters(parameters).map_err(|error| format!("failed to request encoder key frame: {error}"))?;
        }
        let mut buffer = match self.codec.dequeue_input_buffer(INPUT_WAIT).map_err(|error| format!("failed to dequeue encoder input buffer: {error}"))? {
            DequeuedInputBufferResult::Buffer(buffer) => buffer,
            DequeuedInputBufferResult::TryAgainLater => return Err("encoder input buffer unavailable".to_string()),
        };
        let target = buffer.buffer_mut();
        let need = y.len() + uv.len();
        if target.len() < need {
            return Err(format!("encoder input buffer too small: need {need} have {}", target.len()));
        }
        unsafe {
            let destination = target.as_mut_ptr().cast::<u8>();
            std::ptr::copy_nonoverlapping(y.as_ptr(), destination, y.len());
            std::ptr::copy_nonoverlapping(uv.as_ptr(), destination.add(y.len()), uv.len());
        }
        self.codec.queue_input_buffer(buffer, 0, need, timestamp_ms * 1000, 0).map_err(|error| format!("failed to queue encoder input buffer: {error}"))
    }

    pub fn take_output(&mut self, wait: Duration) -> Result<Option<Vec<u8>>, String> {
        loop {
            let output = match self.codec.dequeue_output_buffer(wait).map_err(|error| format!("failed to dequeue encoder output buffer: {error}"))? {
                DequeuedOutputBufferInfoResult::Buffer(output) => output,
                DequeuedOutputBufferInfoResult::TryAgainLater => return Ok(None),
                DequeuedOutputBufferInfoResult::OutputFormatChanged | DequeuedOutputBufferInfoResult::OutputBuffersChanged => continue,
            };
            let info = output.info();
            let offset = usize::try_from(info.offset()).map_err(|error| format!("encoder output offset invalid: {} ({error})", info.offset()))?;
            let size = usize::try_from(info.size()).map_err(|error| format!("encoder output size invalid: {} ({error})", info.size()))?;
            let flags = info.flags();
            let data = output.buffer();
            if offset + size > data.len() {
                return Err(format!("encoder output out of bounds: offset {offset} size {size} len {}", data.len()));
            }
            let bytes = &data[offset..offset + size];
            if flags & BUFFER_FLAG_CODEC_CONFIG != 0 {
                self.parameter_sets = bytes.to_vec();
                self.codec.release_output_buffer(output, false).map_err(|error| format!("failed to release encoder output buffer: {error}"))?;
                continue;
            }
            let mut frame = Vec::with_capacity(self.parameter_sets.len() + size);
            if flags & BUFFER_FLAG_KEY_FRAME != 0 {
                if self.parameter_sets.is_empty() {
                    return Err("encoder produced key frame before parameter sets".to_string());
                }
                frame.extend_from_slice(&self.parameter_sets);
            }
            frame.extend_from_slice(bytes);
            self.codec.release_output_buffer(output, false).map_err(|error| format!("failed to release encoder output buffer: {error}"))?;
            return Ok(Some(frame));
        }
    }
}

impl Drop for HwEncoder {
    fn drop(&mut self) {
        let _ = self.codec.stop();
    }
}

unsafe impl Send for HwEncoder {}
