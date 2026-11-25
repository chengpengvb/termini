use crate::core::error::{RdpProtocolError, RdpResult};

pub fn encode(rgba: &[u8], width: u16, height: u16) -> RdpResult<Vec<u8>> {
    encode_with(rgba, width, height, png::Compression::Default)
}

pub fn encode_fast(rgba: &[u8], width: u16, height: u16) -> RdpResult<Vec<u8>> {
    encode_with(rgba, width, height, png::Compression::Fast)
}

fn encode_with(rgba: &[u8], width: u16, height: u16, compression: png::Compression) -> RdpResult<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol(format!("PNG encode size is zero: {}x{}", width, height)));
    }
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(RdpProtocolError::protocol(format!("PNG encoder pixel length mismatch: need {} have {}", expected, rgba.len())));
    }
    let mut output = Vec::new();
    let mut encoder = png::Encoder::new(&mut output, u32::from(width), u32::from(height));
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(compression);
    let mut writer = encoder.write_header().map_err(|error| RdpProtocolError::encoding(format!("PNG header encode failed: {}", error)))?;
    writer.write_image_data(rgba).map_err(|error| RdpProtocolError::encoding(format!("PNG encode failed: {}", error)))?;
    writer.finish().map_err(|error| RdpProtocolError::encoding(format!("PNG encode finish failed: {}", error)))?;
    Ok(output)
}
