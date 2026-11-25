use std::io::Cursor;
use jpeg_decoder::{Decoder, PixelFormat};
use crate::core::error::{RdpProtocolError, RdpResult};

pub fn decode(source: &[u8], width: usize, height: usize) -> RdpResult<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol(format!("JPEG bitmap size is zero: {}x{}", width, height)));
    }
    let mut decoder = Decoder::new(Cursor::new(source));
    let pixels = decoder.decode().map_err(|error| RdpProtocolError::protocol(format!("JPEG decode failed: {}", error)))?;
    let info = decoder.info().ok_or_else(|| RdpProtocolError::protocol("JPEG image info missing".to_string()))?;
    if info.width as usize != width || info.height as usize != height {
        return Err(RdpProtocolError::protocol(format!("JPEG size mismatch: {}x{} vs {}x{}", info.width, info.height, width, height)));
    }
    if info.pixel_format != PixelFormat::RGB24 {
        return Err(RdpProtocolError::protocol(format!("JPEG is not 24bpp: {:?}", info.pixel_format)));
    }
    let stride = width * 3;
    if pixels.len() < stride * height {
        return Err(RdpProtocolError::underrun(format!("JPEG pixel underrun: need {} have {}", stride * height, pixels.len())));
    }
    let mut output = vec![0u8; width * height * 4];
    for row in 0..height {
        let origin = row * stride;
        let target_row = (height - 1 - row) * width * 4;
        for column in 0..width {
            let source_index = origin + column * 3;
            let target = target_row + column * 4;
            output[target] = pixels[source_index];
            output[target + 1] = pixels[source_index + 1];
            output[target + 2] = pixels[source_index + 2];
            output[target + 3] = 0xFF;
        }
    }
    Ok(output)
}

pub fn encode(rgba: &[u8], width: u16, height: u16, quality: u8) -> RdpResult<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol(format!("JPEG encode size is zero: {}x{}", width, height)));
    }
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(RdpProtocolError::protocol(format!("JPEG encoder pixel length mismatch: need {} have {}", expected, rgba.len())));
    }
    let mut output = Vec::new();
    jpeg_encoder::Encoder::new(&mut output, quality)
        .encode(rgba, width, height, jpeg_encoder::ColorType::Rgba)
        .map_err(|error| RdpProtocolError::encoding(format!("JPEG encode failed: {}", error)))?;
    Ok(output)
}
