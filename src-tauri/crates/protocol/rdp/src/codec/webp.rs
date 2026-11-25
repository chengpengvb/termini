use crate::core::error::{RdpProtocolError, RdpResult};
use libwebp_sys::{
    WebPConfigInitInternal, WebPDecodeRGBA, WebPEncode, WebPFree, WebPMemoryWrite, WebPMemoryWriter, WebPMemoryWriterClear, WebPMemoryWriterInit, WebPPicture, WebPPictureFree, WebPPictureImportRGBA,
    WebPPictureInit, WebPPreset, WebPValidateConfig, WEBP_ENCODER_ABI_VERSION,
};
use std::os::raw::{c_int, c_void};

pub fn encode(rgba: Vec<u8>, width: u16, height: u16, quality: u8) -> RdpResult<Vec<u8>> {
    encode_tuned(rgba, width, height, quality, 2, 1)
}

pub fn encode_with_method(rgba: Vec<u8>, width: u16, height: u16, quality: u8, method: u8) -> RdpResult<Vec<u8>> {
    encode_tuned(rgba, width, height, quality, method, 1)
}

pub fn encode_tuned(rgba: Vec<u8>, width: u16, height: u16, quality: u8, method: u8, thread_level: u8) -> RdpResult<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol(format!("WebP encode size is zero: {}x{}", width, height)));
    }
    if method > 6 {
        return Err(RdpProtocolError::protocol(format!("WebP method outside 0..=6: {}", method)));
    }
    if thread_level > 1 {
        return Err(RdpProtocolError::protocol(format!("WebP thread_level outside 0..=1: {}", thread_level)));
    }
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(RdpProtocolError::protocol(format!("WebP encoder pixel length mismatch: need {} have {}", expected, rgba.len())));
    }
    unsafe {
        let mut config = std::mem::MaybeUninit::uninit();
        if WebPConfigInitInternal(config.as_mut_ptr(), WebPPreset::WEBP_PRESET_DEFAULT, f32::from(quality), WEBP_ENCODER_ABI_VERSION as c_int) == 0 {
            return Err(RdpProtocolError::encoding("WebPConfigPreset failed".to_string()));
        }
        let mut config = config.assume_init();
        config.lossless = 0;
        config.quality = f32::from(quality);
        config.thread_level = c_int::from(thread_level);
        config.method = c_int::from(method);
        if WebPValidateConfig(&config) == 0 {
            return Err(RdpProtocolError::encoding(format!("WebPValidateConfig failed method={}", method)));
        }
        let mut picture = std::mem::MaybeUninit::<WebPPicture>::uninit();
        if !WebPPictureInit(picture.as_mut_ptr()) {
            return Err(RdpProtocolError::encoding("WebPPictureInit failed".to_string()));
        }
        let mut picture = picture.assume_init();
        picture.use_argb = 1;
        picture.width = c_int::from(width);
        picture.height = c_int::from(height);
        if WebPPictureImportRGBA(&mut picture, rgba.as_ptr(), c_int::from(width) * 4) == 0 {
            WebPPictureFree(&mut picture);
            return Err(RdpProtocolError::encoding("WebPPictureImportRGBA failed".to_string()));
        }
        let mut writer = std::mem::MaybeUninit::<WebPMemoryWriter>::uninit();
        WebPMemoryWriterInit(writer.as_mut_ptr());
        let mut writer = writer.assume_init();
        picture.writer = Some(WebPMemoryWrite);
        picture.custom_ptr = (&mut writer as *mut WebPMemoryWriter).cast::<c_void>();
        let ok = WebPEncode(&config, &mut picture);
        let error_code = picture.error_code;
        WebPPictureFree(&mut picture);
        if ok == 0 {
            WebPMemoryWriterClear(&mut writer);
            return Err(RdpProtocolError::encoding(format!("WebPEncode failed: {:?}", error_code)));
        }
        let bytes = std::slice::from_raw_parts(writer.mem, writer.size).to_vec();
        WebPMemoryWriterClear(&mut writer);
        Ok(bytes)
    }
}

pub fn decode(bytes: &[u8]) -> RdpResult<(u16, u16, Vec<u8>)> {
    let mut width = 0 as c_int;
    let mut height = 0 as c_int;
    let ptr = unsafe { WebPDecodeRGBA(bytes.as_ptr(), bytes.len(), &mut width, &mut height) };
    if ptr.is_null() {
        return Err(RdpProtocolError::protocol("WebP decode failed".to_string()));
    }
    let width_u = u16::try_from(width).map_err(|_| RdpProtocolError::protocol(format!("WebP width overflow: {}", width)))?;
    let height_u = u16::try_from(height).map_err(|_| RdpProtocolError::protocol(format!("WebP height overflow: {}", height)))?;
    let len = width as usize * height as usize * 4;
    let pixels = unsafe { std::slice::from_raw_parts(ptr, len).to_vec() };
    unsafe { WebPFree(ptr.cast::<c_void>()) };
    Ok((width_u, height_u, pixels))
}
