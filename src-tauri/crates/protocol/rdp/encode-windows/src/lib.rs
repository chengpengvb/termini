#![cfg(target_os = "windows")]

use std::ptr::null_mut;
use std::sync::OnceLock;
use std::mem::ManuallyDrop;
use std::time::{Duration, Instant};
use windows::core::{Interface, PWSTR};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Variant::{VARIANT, VT_BOOL, VT_UI4};
use windows::Win32::System::Com::{CoIncrementMTAUsage, CoTaskMemFree};
use windows::Win32::Foundation::{E_NOTIMPL, VARIANT_FALSE, VARIANT_TRUE};
use yuv::{rgba_to_yuv_nv12, YuvBiPlanarImageMut, YuvChromaSubsampling, YuvConversionMode, YuvRange, YuvStandardMatrix};

const GOP_FRAMES: u32 = 600;
const FRAME_RATE_HZ: u64 = 60;
const BITRATE_BPS: u32 = 12_000_000;
const SAMPLE_DURATION_100NS: i64 = 166_667;
const EVENT_TIMEOUT: Duration = Duration::from_millis(2000);

static MF_INIT: OnceLock<Result<(), String>> = OnceLock::new();

fn ensure_media_foundation() -> Result<(), String> {
    MF_INIT
        .get_or_init(|| unsafe {
            CoIncrementMTAUsage().map_err(|error| format!("COM MTA initialization failed: {error}"))?;
            MFStartup((MF_SDK_VERSION << 16) | MF_API_VERSION, MFSTARTUP_FULL).map_err(|error| format!("MediaFoundation startup failed: {error}"))?;
            Ok(())
        })
        .clone()
}

fn variant_u32(value: u32) -> VARIANT {
    let mut variant = VARIANT::default();
    unsafe {
        let inner = &mut variant.Anonymous.Anonymous;
        inner.vt = VT_UI4;
        inner.Anonymous.ulVal = value;
    }
    variant
}

fn variant_bool(value: bool) -> VARIANT {
    let mut variant = VARIANT::default();
    unsafe {
        let inner = &mut variant.Anonymous.Anonymous;
        inner.vt = VT_BOOL;
        inner.Anonymous.boolVal = if value { VARIANT_TRUE } else { VARIANT_FALSE };
    }
    variant
}

fn contains_sps(data: &[u8]) -> bool {
    let mut index = 0usize;
    while index + 3 < data.len() {
        let nal = if data[index] == 0 && data[index + 1] == 0 && data[index + 2] == 1 {
            index + 3
        } else if index + 4 <= data.len() && data[index] == 0 && data[index + 1] == 0 && data[index + 2] == 0 && data[index + 3] == 1 {
            index + 4
        } else {
            index += 1;
            continue;
        };
        if nal < data.len() && data[nal] & 0x1f == 7 {
            return true;
        }
        index = nal;
    }
    false
}

pub struct HwEncoder {
    width: u16,
    height: u16,
    name: String,
    input_id: u32,
    output_id: u32,
    outputs_ready: u32,
    input_credits: u32,
    codec_api: ICodecAPI,
    transform: IMFTransform,
    sequence_header: Vec<u8>,
    events: IMFMediaEventGenerator,
    nv12: YuvBiPlanarImageMut<'static, u8>,
}

impl HwEncoder {
    pub fn new(width: u16, height: u16) -> Result<Self, String> {
        ensure_media_foundation()?;
        let (transform, name) = enumerate_hardware_encoder()?;
        unsafe {
            let attributes = transform.GetAttributes().map_err(|error| format!("failed to get encoder attributes: {error}"))?;
            attributes.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1).map_err(|error| format!("failed to unlock async encoder: {error}"))?;

            let mut input_ids = [0u32; 1];
            let mut output_ids = [0u32; 1];
            let (input_id, output_id) = match transform.GetStreamIDs(&mut input_ids, &mut output_ids) {
                Ok(()) => (input_ids[0], output_ids[0]),
                Err(error) if error.code() == E_NOTIMPL => (0u32, 0u32),
                Err(error) => return Err(format!("failed to get encoder stream ids: {error}")),
            };

            let output_type = MFCreateMediaType().map_err(|error| format!("failed to create output type: {error}"))?;
            output_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|error| format!("failed to set output major type: {error}"))?;
            output_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).map_err(|error| format!("failed to set output subtype: {error}"))?;
            output_type.SetUINT32(&MF_MT_AVG_BITRATE, BITRATE_BPS).map_err(|error| format!("failed to set bitrate: {error}"))?;
            output_type.SetUINT64(&MF_MT_FRAME_SIZE, ((width as u64) << 32) | height as u64).map_err(|error| format!("failed to set output frame size: {error}"))?;
            output_type.SetUINT64(&MF_MT_FRAME_RATE, (FRAME_RATE_HZ << 32) | 1).map_err(|error| format!("failed to set output frame rate: {error}"))?;
            output_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(|error| format!("failed to set progressive interlace mode: {error}"))?;
            output_type.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_Main.0 as u32).map_err(|error| format!("failed to set encoder profile: {error}"))?;
            transform.SetOutputType(output_id, &output_type, 0).map_err(|error| format!("failed to apply output type: {error}"))?;

            let input_type = MFCreateMediaType().map_err(|error| format!("failed to create input type: {error}"))?;
            input_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|error| format!("failed to set input major type: {error}"))?;
            input_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12).map_err(|error| format!("failed to set input subtype: {error}"))?;
            input_type.SetUINT64(&MF_MT_FRAME_SIZE, ((width as u64) << 32) | height as u64).map_err(|error| format!("failed to set input frame size: {error}"))?;
            input_type.SetUINT64(&MF_MT_FRAME_RATE, (FRAME_RATE_HZ << 32) | 1).map_err(|error| format!("failed to set input frame rate: {error}"))?;
            input_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(|error| format!("failed to set input interlace mode: {error}"))?;
            transform.SetInputType(input_id, &input_type, 0).map_err(|error| format!("failed to apply input type: {error}"))?;

            let codec_api: ICodecAPI = transform.cast().map_err(|error| format!("failed to get codec control interface: {error}"))?;
            codec_api.SetValue(&CODECAPI_AVEncCommonRateControlMode, &variant_u32(eAVEncCommonRateControlMode_CBR.0 as u32)).map_err(|error| format!("failed to set rate control mode: {error}"))?;
            codec_api.SetValue(&CODECAPI_AVEncCommonMeanBitRate, &variant_u32(BITRATE_BPS)).map_err(|error| format!("failed to set mean bitrate: {error}"))?;
            codec_api.SetValue(&CODECAPI_AVLowLatencyMode, &variant_bool(true)).map_err(|error| format!("failed to set low latency mode: {error}"))?;
            codec_api.SetValue(&CODECAPI_AVEncMPVGOPSize, &variant_u32(GOP_FRAMES)).map_err(|error| format!("failed to set keyframe interval: {error}"))?;

            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0).map_err(|error| format!("failed to notify begin streaming: {error}"))?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0).map_err(|error| format!("failed to notify start of stream: {error}"))?;

            let events: IMFMediaEventGenerator = transform.cast().map_err(|error| format!("failed to get encoder event interface: {error}"))?;
            let nv12 = YuvBiPlanarImageMut::<u8>::alloc(width as u32, height as u32, YuvChromaSubsampling::Yuv420);
            Ok(Self { transform, events, codec_api, nv12, name, width, height, input_id, output_id, input_credits: 0, outputs_ready: 0, sequence_header: Vec::new() })
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn encode(&mut self, rgba: &[u8], timestamp_ms: u64, force_idr: bool) -> Result<Vec<u8>, String> {
        let expected = self.width as usize * self.height as usize * 4;
        if rgba.len() != expected {
            return Err(format!("H264 encoder pixel length mismatch: need {} have {}", expected, rgba.len()));
        }
        rgba_to_yuv_nv12(&mut self.nv12, rgba, self.width as u32 * 4, YuvRange::Limited, YuvStandardMatrix::Bt601, YuvConversionMode::Balanced)
            .map_err(|error| format!("RGBA to NV12 conversion failed: {error}"))?;
        if force_idr {
            unsafe { self.codec_api.SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &variant_u32(1)) }.map_err(|error| format!("failed to request keyframe: {error}"))?;
        }
        let sample = self.build_input_sample(timestamp_ms)?;
        let deadline = Instant::now() + EVENT_TIMEOUT;
        while self.input_credits == 0 {
            self.pump_event(deadline)?;
        }
        unsafe { self.transform.ProcessInput(self.input_id, &sample, 0) }.map_err(|error| format!("failed to submit encoder input: {error}"))?;
        self.input_credits -= 1;
        while self.outputs_ready == 0 {
            self.pump_event(deadline)?;
        }
        self.outputs_ready -= 1;
        let (bytes, keyframe) = self.drain_output()?;
        if bytes.is_empty() {
            return Err("H264 encoder produced no output".to_string());
        }
        self.ensure_parameter_sets(bytes, keyframe)
    }

    pub fn encode_nv12(&mut self, y: &[u8], uv: &[u8], timestamp_ms: u64, force_idr: bool) -> Result<Vec<u8>, String> {
        let expected_y = self.width as usize * self.height as usize;
        let expected_uv = self.width as usize * (self.height as usize).div_ceil(2);
        if y.len() != expected_y || uv.len() != expected_uv {
            return Err(format!("H264 encoder NV12 length mismatch: y need {} have {} uv need {} have {}", expected_y, y.len(), expected_uv, uv.len()));
        }
        if force_idr {
            unsafe { self.codec_api.SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &variant_u32(1)) }.map_err(|error| format!("failed to request keyframe: {error}"))?;
        }
        let sample = self.build_input_sample_from_nv12(y, uv, timestamp_ms)?;
        let deadline = Instant::now() + EVENT_TIMEOUT;
        while self.input_credits == 0 {
            self.pump_event(deadline)?;
        }
        unsafe { self.transform.ProcessInput(self.input_id, &sample, 0) }.map_err(|error| format!("failed to submit encoder input: {error}"))?;
        self.input_credits -= 1;
        while self.outputs_ready == 0 {
            self.pump_event(deadline)?;
        }
        self.outputs_ready -= 1;
        let (bytes, keyframe) = self.drain_output()?;
        if bytes.is_empty() {
            return Err("H264 encoder produced no output".to_string());
        }
        self.ensure_parameter_sets(bytes, keyframe)
    }

    pub fn set_bitrate(&mut self, bitrate: u32) -> Result<(), String> {
        unsafe { self.codec_api.SetValue(&CODECAPI_AVEncCommonMeanBitRate, &variant_u32(bitrate)) }.map_err(|error| format!("failed to set mean bitrate: {error}"))
    }

    fn pump_event(&mut self, deadline: Instant) -> Result<(), String> {
        loop {
            match unsafe { self.events.GetEvent(MF_EVENT_FLAG_NO_WAIT) } {
                Ok(event) => {
                    let kind = unsafe { event.GetType() }.map_err(|error| format!("failed to read encoder event type: {error}"))?;
                    if kind == METransformNeedInput.0 as u32 {
                        self.input_credits += 1;
                    } else if kind == METransformHaveOutput.0 as u32 {
                        self.outputs_ready += 1;
                    }
                    return Ok(());
                }
                Err(error) if error.code() == MF_E_NO_EVENTS_AVAILABLE => {
                    if Instant::now() >= deadline {
                        return Err("timed out waiting for hardware encoder event".to_string());
                    }
                    std::thread::sleep(Duration::from_micros(500));
                }
                Err(error) => return Err(format!("failed to get encoder event: {error}")),
            }
        }
    }

    fn build_input_sample(&self, timestamp_ms: u64) -> Result<IMFSample, String> {
        let y_plane = self.nv12.y_plane.borrow();
        let uv_plane = self.nv12.uv_plane.borrow();
        self.build_input_sample_from_nv12(&y_plane, &uv_plane, timestamp_ms)
    }

    fn build_input_sample_from_nv12(&self, y: &[u8], uv: &[u8], timestamp_ms: u64) -> Result<IMFSample, String> {
        let total = y.len() + uv.len();
        unsafe {
            let media_buffer = MFCreateMemoryBuffer(total as u32).map_err(|error| format!("failed to create input buffer: {error}"))?;
            let mut target = null_mut();
            media_buffer.Lock(&mut target, None, None).map_err(|error| format!("failed to lock input buffer: {error}"))?;
            std::ptr::copy_nonoverlapping(y.as_ptr(), target, y.len());
            std::ptr::copy_nonoverlapping(uv.as_ptr(), target.add(y.len()), uv.len());
            media_buffer.Unlock().map_err(|error| format!("failed to unlock input buffer: {error}"))?;
            media_buffer.SetCurrentLength(total as u32).map_err(|error| format!("failed to set input length: {error}"))?;
            let sample = MFCreateSample().map_err(|error| format!("failed to create input sample: {error}"))?;
            sample.AddBuffer(&media_buffer).map_err(|error| format!("failed to attach input buffer: {error}"))?;
            sample.SetSampleTime((timestamp_ms * 10_000) as i64).map_err(|error| format!("failed to set sample time: {error}"))?;
            sample.SetSampleDuration(SAMPLE_DURATION_100NS).map_err(|error| format!("failed to set sample duration: {error}"))?;
            Ok(sample)
        }
    }

    fn drain_output(&mut self) -> Result<(Vec<u8>, bool), String> {
        unsafe {
            let mut buffers = [MFT_OUTPUT_DATA_BUFFER { dwStreamID: self.output_id, pSample: ManuallyDrop::new(None), dwStatus: 0, pEvents: ManuallyDrop::new(None) }];
            let mut status = 0u32;
            self.transform.ProcessOutput(0, &mut buffers, &mut status).map_err(|error| format!("failed to read encoder output: {error}"))?;
            let sample = ManuallyDrop::take(&mut buffers[0].pSample).ok_or_else(|| "encoder output sample is null".to_string())?;
            drop(ManuallyDrop::take(&mut buffers[0].pEvents));
            let keyframe = match sample.GetUINT32(&MFSampleExtension_CleanPoint) {
                Ok(value) => value == 1,
                Err(error) if error.code() == MF_E_ATTRIBUTENOTFOUND => false,
                Err(error) => return Err(format!("failed to read keyframe flag: {error}")),
            };
            let contiguous = sample.ConvertToContiguousBuffer().map_err(|error| format!("failed to merge output buffers: {error}"))?;
            let mut data = null_mut();
            let mut current = 0u32;
            contiguous.Lock(&mut data, None, Some(&mut current)).map_err(|error| format!("failed to lock output buffer: {error}"))?;
            let bytes = std::slice::from_raw_parts(data, current as usize).to_vec();
            contiguous.Unlock().map_err(|error| format!("failed to unlock output buffer: {error}"))?;
            Ok((bytes, keyframe))
        }
    }

    fn ensure_parameter_sets(&mut self, bytes: Vec<u8>, keyframe: bool) -> Result<Vec<u8>, String> {
        if !keyframe || contains_sps(&bytes) {
            return Ok(bytes);
        }
        if self.sequence_header.is_empty() {
            self.sequence_header = self.query_sequence_header()?;
        }
        if self.sequence_header.is_empty() {
            return Err("IDR output lacks SPS/PPS and the encoder provided no sequence header".to_string());
        }
        let mut out = Vec::with_capacity(self.sequence_header.len() + bytes.len());
        out.extend_from_slice(&self.sequence_header);
        out.extend_from_slice(&bytes);
        Ok(out)
    }

    fn query_sequence_header(&self) -> Result<Vec<u8>, String> {
        unsafe {
            let current = self.transform.GetOutputCurrentType(self.output_id).map_err(|error| format!("failed to read output type: {error}"))?;
            let size = match current.GetBlobSize(&MF_MT_MPEG_SEQUENCE_HEADER) {
                Ok(size) => size,
                Err(error) if error.code() == MF_E_ATTRIBUTENOTFOUND => return Ok(Vec::new()),
                Err(error) => return Err(format!("failed to read sequence header length: {error}")),
            };
            let mut blob = vec![0u8; size as usize];
            current.GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut blob, None).map_err(|error| format!("failed to read sequence header: {error}"))?;
            Ok(blob)
        }
    }
}

impl Drop for HwEncoder {
    fn drop(&mut self) {
        unsafe {
            let _ = self.transform.ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0);
            let _ = self.transform.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0);
        }
    }
}

unsafe impl Send for HwEncoder {}

fn enumerate_hardware_encoder() -> Result<(IMFTransform, String), String> {
    unsafe {
        let output_info = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Video, guidSubtype: MFVideoFormat_H264 };
        let mut activates: *mut Option<IMFActivate> = null_mut();
        let mut count = 0u32;
        MFTEnumEx(MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG(MFT_ENUM_FLAG_HARDWARE.0 | MFT_ENUM_FLAG_SORTANDFILTER.0), None, Some(&output_info), &mut activates, &mut count)
            .map_err(|error| format!("failed to enumerate hardware encoders: {error}"))?;
        if count == 0 || activates.is_null() {
            if !activates.is_null() {
                CoTaskMemFree(Some(activates as *const _));
            }
            return Err("no hardware H264 encoder found".to_string());
        }
        let slice = std::slice::from_raw_parts_mut(activates, count as usize);
        let first = slice[0].take();
        for item in slice.iter_mut().skip(1) {
            drop(item.take());
        }
        CoTaskMemFree(Some(activates as *const _));
        let activate = first.ok_or_else(|| "hardware encoder activation handle is null".to_string())?;
        let mut pwstr = PWSTR::null();
        let mut length = 0u32;
        activate.GetAllocatedString(&MFT_FRIENDLY_NAME_Attribute, &mut pwstr, &mut length).map_err(|error| format!("failed to read encoder name: {error}"))?;
        let name = pwstr.to_string().map_err(|error| format!("failed to decode encoder name: {error}"))?;
        CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
        let transform: IMFTransform = activate.ActivateObject().map_err(|error| format!("failed to activate hardware encoder: {error}"))?;
        Ok((transform, name))
    }
}
