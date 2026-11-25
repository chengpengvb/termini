use openh264::Timestamp;
use std::time::Duration;
use openh264::OpenH264API;
use crate::gdi::framebuffer::Rect;
use crate::core::stream::ReadCursor;
use openh264::decoder::{Decoder, DecoderConfig};
use crate::core::error::{RdpProtocolError, RdpResult};
use std::ptr::{addr_of_mut, from_mut, null, null_mut};
use openh264::formats::{RgbaSliceU8, YUVBuffer, YUVSlices};
use openh264::encoder::{BitRate, Complexity, Encoder, EncoderConfig, FrameRate, RateControlMode, UsageType, VuiConfig};
use openh264_sys2::{dsErrorFree, dsNoParamSets, videoFormatI420, SBufferInfo, DECODER_OPTION_ERROR_CON_IDC, ERROR_CON_FRAME_COPY};

const OUTPUT_MAX_FPS: f32 = 60.0;
const OUTPUT_BITRATE_BPS: u32 = 12_000_000;

pub struct Avc420Stream<'a> {
    pub rects: Vec<Rect>,
    pub bitstream: &'a [u8],
}

pub fn parse_avc420(data: &[u8]) -> RdpResult<Avc420Stream<'_>> {
    let mut cursor = ReadCursor::new(data);
    let remaining = cursor.remaining();
    parse_avc420_cursor(&mut cursor, remaining)
}

pub fn require_annexb(bitstream: &[u8]) -> RdpResult<()> {
    if bitstream.starts_with(&[0, 0, 0, 1]) || bitstream.starts_with(&[0, 0, 1]) {
        return Ok(());
    }
    Err(RdpProtocolError::encoding(format!("AVC420 bitstream is not Annex-B: length {}", bitstream.len())))
}

pub fn parse_avc444(data: &[u8]) -> RdpResult<(u8, Avc420Stream<'_>, Option<Avc420Stream<'_>>)> {
    let mut cursor = ReadCursor::new(data);
    let packed = cursor.read_u32_le()?;
    let size1 = (packed & 0x3FFF_FFFF) as usize;
    let lc = ((packed >> 30) & 0x03) as u8;
    if lc == 3 {
        return Err(RdpProtocolError::protocol(format!("AVC444 invalid LC: {}", lc)));
    }
    let first_start = cursor.remaining();
    let header_consumed = data.len() - first_start;
    let first_limit = if lc == 0 {
        if size1 > data.len().saturating_sub(header_consumed) {
            return Err(RdpProtocolError::protocol(format!("AVC444 primary stream length out of bounds: {}", size1)));
        }
        size1
    } else {
        cursor.remaining()
    };
    let luma = parse_avc420_cursor(&mut cursor, first_limit)?;
    if lc != 0 {
        return Ok((lc, luma, None));
    }
    let remaining = cursor.remaining();
    let chroma = parse_avc420_cursor(&mut cursor, remaining)?;
    Ok((lc, luma, Some(chroma)))
}

fn parse_avc420_cursor<'a>(cursor: &mut ReadCursor<'a>, limit: usize) -> RdpResult<Avc420Stream<'a>> {
    let remaining_before = cursor.remaining();
    if limit > remaining_before {
        return Err(RdpProtocolError::protocol(format!("AVC420 length out of bounds: limit {} remaining {}", limit, remaining_before)));
    }
    let count = cursor.read_u32_le()?;
    let mut rects = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let left = cursor.read_u16_le()?;
        let top = cursor.read_u16_le()?;
        let right = cursor.read_u16_le()?;
        let bottom = cursor.read_u16_le()?;
        rects.push(Rect::new(left, top, right, bottom)?);
    }
    cursor.skip(count as usize * 2)?;
    let consumed = remaining_before - cursor.remaining();
    if consumed > limit {
        return Err(RdpProtocolError::protocol(format!("AVC420 metadata exceeds bitstream: consumed {} limit {}", consumed, limit)));
    }
    let bitstream_len = limit - consumed;
    let bitstream = cursor.read_slice(bitstream_len)?;
    Ok(Avc420Stream { rects, bitstream })
}

pub const AVC_FLAG_V2: u8 = 0x01;
pub const AVC_FLAG_444: u8 = 0x02;

pub fn pack_avc420_passthrough(data: &[u8]) -> RdpResult<Vec<u8>> {
    let stream = parse_avc420(data)?;
    let mut payload = Vec::with_capacity(data.len() + 16);
    payload.push(0);
    write_stream_segment(&mut payload, Some(&stream))?;
    write_stream_segment(&mut payload, None)?;
    Ok(payload)
}

pub fn pack_avc444_passthrough(v2: bool, data: &[u8]) -> RdpResult<Vec<u8>> {
    let (lc, first, second) = parse_avc444(data)?;
    let (luma, chroma) = match lc {
        0 => (Some(&first), second.as_ref()),
        1 => (Some(&first), None),
        _ => (None, Some(&first)),
    };
    let mut payload = Vec::with_capacity(data.len() + 32);
    payload.push(AVC_FLAG_444 | if v2 { AVC_FLAG_V2 } else { 0 });
    write_stream_segment(&mut payload, luma)?;
    write_stream_segment(&mut payload, chroma)?;
    Ok(payload)
}

pub struct AvcDecoder {
    inner: Decoder,
}

struct DecodedI420 {
    width: usize,
    height: usize,
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    y_stride: usize,
    uv_stride: usize,
}

impl AvcDecoder {
    pub fn new() -> RdpResult<Self> {
        let mut inner = Decoder::with_api_config(OpenH264API::from_source(), DecoderConfig::new()).map_err(|error| RdpProtocolError::encoding(format!("failed to create H264 decoder: {}", error)))?;
        unsafe {
            let mut concealment = ERROR_CON_FRAME_COPY;
            let status = inner.raw_api().set_option(DECODER_OPTION_ERROR_CON_IDC, addr_of_mut!(concealment).cast());
            if status != 0 {
                return Err(RdpProtocolError::encoding(format!("failed to enable H264 error concealment: {}", status)));
            }
        }
        Ok(Self { inner })
    }

    pub fn blit(&mut self, bitstream: &[u8], pixels: &mut [u8], stride: usize, surface_width: usize, surface_height: usize, rects: &[Rect]) -> RdpResult<bool> {
        require_annexb(bitstream)?;
        let Some(frame) = self.decode_i420(bitstream) else {
            return Ok(false);
        };
        let mut wrote = false;
        for rect in rects {
            wrote |= blit_i420_rect(&frame, pixels, stride, surface_width, surface_height, *rect);
        }
        Ok(wrote)
    }

    fn decode_i420(&mut self, bitstream: &[u8]) -> Option<DecodedI420> {
        unsafe {
            let mut dst = [null_mut::<u8>(); 3];
            let mut info = SBufferInfo::default();
            let state = self.inner.raw_api().decode_frame2(bitstream.as_ptr(), bitstream.len() as i32, from_mut(&mut dst).cast(), &mut info);
            if info.iBufferStatus != 1 && (state == dsErrorFree || state == dsNoParamSets) {
                dst = [null_mut(); 3];
                info = SBufferInfo::default();
                let _ = self.inner.raw_api().decode_frame2(null(), 0, from_mut(&mut dst).cast(), &mut info);
            }
            if info.iBufferStatus != 1 {
                dst = [null_mut(); 3];
                info = SBufferInfo::default();
                let _ = self.inner.raw_api().flush_frame(from_mut(&mut dst).cast(), &mut info);
            }
            copy_i420(dst, &info)
        }
    }
}

fn copy_i420(dst: [*mut u8; 3], info: &SBufferInfo) -> Option<DecodedI420> {
    if info.iBufferStatus != 1 || dst[0].is_null() || dst[1].is_null() || dst[2].is_null() {
        return None;
    }
    let sys = unsafe { info.UsrData.sSystemBuffer };
    if sys.iFormat != videoFormatI420 || sys.iWidth <= 0 || sys.iHeight <= 0 || sys.iStride[0] <= 0 || sys.iStride[1] <= 0 {
        return None;
    }
    let width = sys.iWidth as usize;
    let height = sys.iHeight as usize;
    let y_stride = sys.iStride[0] as usize;
    let uv_stride = sys.iStride[1] as usize;
    let chroma_height = height.div_ceil(2);
    if y_stride < width || uv_stride < width.div_ceil(2) {
        return None;
    }
    Some(DecodedI420 {
        width,
        height,
        y: unsafe { std::slice::from_raw_parts(dst[0], y_stride * height) }.to_vec(),
        u: unsafe { std::slice::from_raw_parts(dst[1], uv_stride * chroma_height) }.to_vec(),
        v: unsafe { std::slice::from_raw_parts(dst[2], uv_stride * chroma_height) }.to_vec(),
        y_stride,
        uv_stride,
    })
}

fn blit_i420_rect(frame: &DecodedI420, pixels: &mut [u8], stride: usize, surface_width: usize, surface_height: usize, rect: Rect) -> bool {
    let left = rect.left as usize;
    let top = rect.top as usize;
    let right = (rect.right as usize).min(frame.width).min(surface_width);
    let bottom = (rect.bottom as usize).min(frame.height).min(surface_height);
    if left >= right || top >= bottom {
        return false;
    }
    for row in top..bottom {
        for column in left..right {
            let y = frame.y[row * frame.y_stride + column];
            let chroma = (row / 2) * frame.uv_stride + column / 2;
            let pixel = yuv_to_rgba(y, frame.u[chroma], frame.v[chroma]);
            let target = row * stride + column * 4;
            pixels[target..target + 4].copy_from_slice(&pixel);
        }
    }
    true
}

fn yuv_to_rgba(y: u8, u: u8, v: u8) -> [u8; 4] {
    let c = i32::from(y) - 16;
    let d = i32::from(u) - 128;
    let e = i32::from(v) - 128;
    [((298 * c + 409 * e + 128) >> 8).clamp(0, 255) as u8, ((298 * c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8, ((298 * c + 516 * d + 128) >> 8).clamp(0, 255) as u8, 0xFF]
}

fn write_stream_segment(payload: &mut Vec<u8>, stream: Option<&Avc420Stream<'_>>) -> RdpResult<()> {
    let Some(stream) = stream else {
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        return Ok(());
    };
    require_annexb(stream.bitstream)?;
    let count = u16::try_from(stream.rects.len()).map_err(|_| RdpProtocolError::protocol(format!("AVC region count out of range: {}", stream.rects.len())))?;
    payload.extend_from_slice(&count.to_le_bytes());
    for rect in &stream.rects {
        payload.extend_from_slice(&rect.left.to_le_bytes());
        payload.extend_from_slice(&rect.top.to_le_bytes());
        payload.extend_from_slice(&rect.width().to_le_bytes());
        payload.extend_from_slice(&rect.height().to_le_bytes());
    }
    let length = u32::try_from(stream.bitstream.len()).map_err(|_| RdpProtocolError::protocol(format!("AVC bitstream too long: {}", stream.bitstream.len())))?;
    payload.extend_from_slice(&length.to_le_bytes());
    payload.extend_from_slice(stream.bitstream);
    Ok(())
}

enum OutputBackend {
    Software(Encoder),
    #[cfg(target_os = "windows")]
    Hardware(encode_windows::HwEncoder),
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    Hardware(encode_apple::HwEncoder),
    #[cfg(target_os = "android")]
    Hardware(encode_android::HwEncoder),
}

pub struct OutputEncoder {
    width: u16,
    height: u16,
    bitrate: u32,
    need_idr: bool,
    pending: usize,
    backend_name: String,
    backend: OutputBackend,
    ready: Option<Vec<u8>>,
}

impl OutputEncoder {
    pub fn new(width: u16, height: u16) -> RdpResult<Self> {
        Self::with_bitrate(width, height, OUTPUT_BITRATE_BPS)
    }

    pub fn with_bitrate(width: u16, height: u16, bitrate: u32) -> RdpResult<Self> {
        if width == 0 || height == 0 {
            return Err(RdpProtocolError::protocol(format!("H264 encode size is zero: {}x{}", width, height)));
        }
        if width % 2 != 0 || height % 2 != 0 {
            return Err(RdpProtocolError::protocol(format!("H264 encode size must be even: {}x{}", width, height)));
        }
        if bitrate == 0 {
            return Err(RdpProtocolError::protocol("H264 encode bitrate is zero".to_string()));
        }
        let (backend, backend_name) = match create_hardware_backend(width, height) {
            Ok(outcome) => outcome,
            Err(reason) => (OutputBackend::Software(create_software_encoder(bitrate)?), format!("openh264 software encoder (hardware unavailable: {reason})")),
        };
        Ok(Self { backend, backend_name, width, height, bitrate, need_idr: false, pending: 0, ready: None })
    }

    pub fn set_bitrate(&mut self, bitrate: u32) -> RdpResult<()> {
        if bitrate == 0 {
            return Err(RdpProtocolError::protocol("H264 encode bitrate is zero".to_string()));
        }
        if self.bitrate == bitrate {
            return Ok(());
        }
        match &mut self.backend {
            OutputBackend::Software(_) => {
                self.backend = OutputBackend::Software(create_software_encoder(bitrate)?);
                self.need_idr = true;
            }
            #[cfg(target_os = "windows")]
            OutputBackend::Hardware(encoder) => {
                encoder.set_bitrate(bitrate).map_err(|error| RdpProtocolError::encoding(error))?;
            }
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            OutputBackend::Hardware(encoder) => {
                encoder.set_bitrate(bitrate).map_err(|error| RdpProtocolError::encoding(error))?;
            }
            #[cfg(target_os = "android")]
            OutputBackend::Hardware(encoder) => {
                encoder.set_bitrate(bitrate).map_err(|error| RdpProtocolError::encoding(error))?;
            }
        }
        self.bitrate = bitrate;
        Ok(())
    }

    pub fn backend_name(&self) -> &str {
        &self.backend_name
    }

    pub fn matches(&self, width: u16, height: u16) -> bool {
        self.width == width && self.height == height
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn mark_idr(&mut self) {
        self.need_idr = true;
    }

    pub fn pending(&self) -> usize {
        self.pending
    }

    pub fn submit(&mut self, rgba: &[u8], timestamp_ms: u64) -> RdpResult<()> {
        let expected = self.width as usize * self.height as usize * 4;
        if rgba.len() != expected {
            return Err(RdpProtocolError::protocol(format!("H264 encoder pixel length mismatch: need {} have {}", expected, rgba.len())));
        }
        let force_idr = self.need_idr;
        self.need_idr = false;
        match &mut self.backend {
            OutputBackend::Software(encoder) => {
                if force_idr {
                    encoder.force_intra_frame();
                }
                let source = RgbaSliceU8::new(rgba, (self.width as usize, self.height as usize));
                let yuv = YUVBuffer::from_rgb_source(source);
                let stream = encoder.encode_at(&yuv, Timestamp::from_millis(timestamp_ms)).map_err(|error| RdpProtocolError::encoding(format!("H264 encode failed: {}", error)))?;
                self.ready = Some(stream.to_vec());
            }
            #[cfg(target_os = "windows")]
            OutputBackend::Hardware(encoder) => {
                let bytes = encoder.encode(rgba, timestamp_ms, force_idr).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 encode failed: {}", error)))?;
                self.ready = Some(bytes);
            }
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            OutputBackend::Hardware(encoder) => {
                encoder.submit(rgba, timestamp_ms, force_idr).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 submit failed: {}", error)))?;
            }
            #[cfg(target_os = "android")]
            OutputBackend::Hardware(encoder) => {
                encoder.submit(rgba, timestamp_ms, force_idr).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 submit failed: {}", error)))?;
            }
        }
        self.pending += 1;
        Ok(())
    }

    pub fn submit_nv12(&mut self, y: &[u8], uv: &[u8], timestamp_ms: u64) -> RdpResult<()> {
        let expected_y = self.width as usize * self.height as usize;
        let expected_uv = self.width as usize * (self.height as usize).div_ceil(2);
        if y.len() != expected_y || uv.len() != expected_uv {
            return Err(RdpProtocolError::protocol(format!("H264 encoder NV12 length mismatch: y need {} have {} uv need {} have {}", expected_y, y.len(), expected_uv, uv.len())));
        }
        let force_idr = self.need_idr;
        self.need_idr = false;
        match &mut self.backend {
            OutputBackend::Software(encoder) => {
                if force_idr {
                    encoder.force_intra_frame();
                }
                let (u, v) = split_nv12_uv(uv, self.width as usize, self.height as usize);
                let source = YUVSlices::new((y, &u, &v), (self.width as usize, self.height as usize), (self.width as usize, self.width as usize / 2, self.width as usize / 2));
                let stream = encoder.encode_at(&source, Timestamp::from_millis(timestamp_ms)).map_err(|error| RdpProtocolError::encoding(format!("H264 NV12 encode failed: {}", error)))?;
                self.ready = Some(stream.to_vec());
            }
            #[cfg(target_os = "windows")]
            OutputBackend::Hardware(encoder) => {
                let bytes = encoder.encode_nv12(y, uv, timestamp_ms, force_idr).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 NV12 encode failed: {}", error)))?;
                self.ready = Some(bytes);
            }
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            OutputBackend::Hardware(encoder) => {
                encoder.submit_nv12(y, uv, timestamp_ms, force_idr).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 NV12 submit failed: {}", error)))?;
            }
            #[cfg(target_os = "android")]
            OutputBackend::Hardware(encoder) => {
                encoder.submit_nv12(y, uv, timestamp_ms, force_idr).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 NV12 submit failed: {}", error)))?;
            }
        }
        self.pending += 1;
        Ok(())
    }

    pub fn take_output(&mut self, wait: Duration) -> RdpResult<Option<Vec<u8>>> {
        if self.pending == 0 {
            return Ok(None);
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
        let _ = wait;
        let output = match &mut self.backend {
            OutputBackend::Software(_) => self.ready.take(),
            #[cfg(target_os = "windows")]
            OutputBackend::Hardware(_) => self.ready.take(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            OutputBackend::Hardware(encoder) => encoder.take_output(wait).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 output failed: {}", error)))?,
            #[cfg(target_os = "android")]
            OutputBackend::Hardware(encoder) => encoder.take_output(wait).map_err(|error| RdpProtocolError::encoding(format!("hardware H264 output failed: {}", error)))?,
        };
        if let Some(bytes) = &output {
            if bytes.is_empty() {
                return Err(RdpProtocolError::encoding("H264 encoder produced no output".to_string()));
            }
            self.pending -= 1;
        }
        Ok(output)
    }
}

fn split_nv12_uv(uv: &[u8], width: usize, height: usize) -> (Vec<u8>, Vec<u8>) {
    let chroma_w = width / 2;
    let chroma_h = height / 2;
    let mut u = vec![0u8; chroma_w * chroma_h];
    let mut v = vec![0u8; chroma_w * chroma_h];
    for row in 0..chroma_h {
        for col in 0..chroma_w {
            let source = row * width + col * 2;
            let dest = row * chroma_w + col;
            u[dest] = uv[source];
            v[dest] = uv[source + 1];
        }
    }
    (u, v)
}

fn create_software_encoder(bitrate: u32) -> RdpResult<Encoder> {
    let config = EncoderConfig::new()
        .usage_type(UsageType::ScreenContentRealTime)
        .rate_control_mode(RateControlMode::Quality)
        .bitrate(BitRate::from_bps(bitrate))
        .max_frame_rate(FrameRate::from_hz(OUTPUT_MAX_FPS))
        .skip_frames(false)
        .complexity(Complexity::Low)
        .vui(VuiConfig::bt601());
    Encoder::with_api_config(OpenH264API::from_source(), config).map_err(|error| RdpProtocolError::encoding(format!("failed to create H264 encoder: {}", error)))
}

#[cfg(target_os = "windows")]
fn create_hardware_backend(width: u16, height: u16) -> Result<(OutputBackend, String), String> {
    let encoder = encode_windows::HwEncoder::new(width, height)?;
    let name = format!("hardware encoder {}", encoder.name());
    Ok((OutputBackend::Hardware(encoder), name))
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn create_hardware_backend(width: u16, height: u16) -> Result<(OutputBackend, String), String> {
    let encoder = encode_apple::HwEncoder::new(width, height)?;
    let name = format!("hardware encoder {}", encoder.name());
    Ok((OutputBackend::Hardware(encoder), name))
}

#[cfg(target_os = "android")]
fn create_hardware_backend(width: u16, height: u16) -> Result<(OutputBackend, String), String> {
    let encoder = encode_android::HwEncoder::new(width, height)?;
    let name = format!("hardware encoder {}", encoder.name());
    Ok((OutputBackend::Hardware(encoder), name))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android")))]
fn create_hardware_backend(_width: u16, _height: u16) -> Result<(OutputBackend, String), String> {
    Err("no hardware encoder implementation on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avc444_passthrough_packs_streams() {
        let bitstream = [0u8, 0, 0, 1, 0x67, 0x42, 0xC0, 0x2A];
        let mut segment = Vec::new();
        segment.extend_from_slice(&1u32.to_le_bytes());
        segment.extend_from_slice(&0u16.to_le_bytes());
        segment.extend_from_slice(&0u16.to_le_bytes());
        segment.extend_from_slice(&64u16.to_le_bytes());
        segment.extend_from_slice(&64u16.to_le_bytes());
        segment.extend_from_slice(&[22, 100]);
        segment.extend_from_slice(&bitstream);
        let mut data = Vec::new();
        data.extend_from_slice(&(segment.len() as u32).to_le_bytes());
        data.extend_from_slice(&segment);
        data.extend_from_slice(&segment);

        let payload = pack_avc444_passthrough(true, &data).expect("pack");
        assert_eq!(payload[0], AVC_FLAG_444 | AVC_FLAG_V2);
        let mut offset = 1;
        for _ in 0..2 {
            let count = u16::from_le_bytes([payload[offset], payload[offset + 1]]) as usize;
            assert_eq!(count, 1);
            offset += 2 + count * 8;
            let length = u32::from_le_bytes([payload[offset], payload[offset + 1], payload[offset + 2], payload[offset + 3]]) as usize;
            assert_eq!(length, bitstream.len());
            offset += 4;
            assert_eq!(&payload[offset..offset + length], &bitstream);
            offset += length;
        }
        assert_eq!(offset, payload.len());
    }

    #[test]
    fn avc420_decodes_encoded_frame_to_surface() {
        let width = 64usize;
        let height = 64usize;
        let mut rgba = vec![0u8; width * height * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[0] = 200;
            pixel[1] = 16;
            pixel[2] = 16;
            pixel[3] = 255;
        }
        let mut encoder = create_software_encoder(OUTPUT_BITRATE_BPS).expect("encoder");
        encoder.force_intra_frame();
        let source = RgbaSliceU8::new(&rgba, (width, height));
        let yuv = YUVBuffer::from_rgb_source(source);
        let bitstream = encoder.encode_at(&yuv, Timestamp::from_millis(0)).expect("encode").to_vec();
        let rect = Rect::from_size(0, 0, width as u16, height as u16).expect("rect");
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&rect.left.to_le_bytes());
        data.extend_from_slice(&rect.top.to_le_bytes());
        data.extend_from_slice(&rect.right.to_le_bytes());
        data.extend_from_slice(&rect.bottom.to_le_bytes());
        data.extend_from_slice(&[22, 100]);
        data.extend_from_slice(&bitstream);
        let mut surfaces = crate::gdi::gfx::GfxSurfaces::default();
        surfaces.create(1, width as u16, height as u16, crate::gdi::gfx::PIXEL_FORMAT_XRGB).expect("create");
        surfaces.map_to_output(1, 0, 0).expect("map");
        surfaces.surface_command(&crate::gdi::gfx::SurfaceCommand { surface_id: 1, codec_id: crate::gdi::gfx::CODEC_AVC420, alpha: false, rect, data: &data }).expect("avc420");
        let mut junk = Vec::new();
        junk.extend_from_slice(&1u32.to_le_bytes());
        junk.extend_from_slice(&rect.left.to_le_bytes());
        junk.extend_from_slice(&rect.top.to_le_bytes());
        junk.extend_from_slice(&rect.right.to_le_bytes());
        junk.extend_from_slice(&rect.bottom.to_le_bytes());
        junk.extend_from_slice(&[22, 100]);
        junk.extend_from_slice(&[0, 0, 0, 1, 0x65, 0, 0, 0, 0]);
        surfaces.surface_command(&crate::gdi::gfx::SurfaceCommand { surface_id: 1, codec_id: crate::gdi::gfx::CODEC_AVC420, alpha: false, rect, data: &junk }).expect("corrupt nal");
        assert!(surfaces.take_avc_patches().is_empty());
        let mut framebuffer = crate::gdi::framebuffer::Framebuffer::new(width as u16, height as u16);
        surfaces.update(&mut framebuffer).expect("update");
        let pixel = framebuffer.copy_rgba_rect(&rect);
        assert!(pixel[0] > 80, "expected red channel, got {:?}", &pixel[..4]);
        assert!(pixel[1] < 120, "expected low green, got {:?}", &pixel[..4]);
        assert!(pixel[2] < 120, "expected low blue, got {:?}", &pixel[..4]);
    }

    #[test]
    fn avc420_passthrough_packs_single_stream() {
        let bitstream = [0u8, 0, 1, 0x65];
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&8u16.to_le_bytes());
        data.extend_from_slice(&8u16.to_le_bytes());
        data.extend_from_slice(&24u16.to_le_bytes());
        data.extend_from_slice(&24u16.to_le_bytes());
        data.extend_from_slice(&[22, 100]);
        data.extend_from_slice(&bitstream);

        let payload = pack_avc420_passthrough(&data).expect("pack");
        assert_eq!(payload[0], 0);
        let count = u16::from_le_bytes([payload[1], payload[2]]) as usize;
        assert_eq!(count, 1);
        let rect = &payload[3..11];
        assert_eq!(u16::from_le_bytes([rect[0], rect[1]]), 8);
        assert_eq!(u16::from_le_bytes([rect[4], rect[5]]), 16);
        let chroma_offset = payload.len() - 6;
        assert_eq!(&payload[chroma_offset..], &[0, 0, 0, 0, 0, 0]);
    }
}
