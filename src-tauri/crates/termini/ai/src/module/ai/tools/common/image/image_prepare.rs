use std::io::Cursor;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};

const MAX_TILES: usize = 12;
const TILE_OVERLAP: u32 = 120;
const TILE_PIXEL_RATIO: f64 = 2.0;
const DEFAULT_JPEG_QUALITY: u8 = 88;
const KEEP_ORIGINAL_BELOW_BYTES: usize = 200 * 1024;

#[derive(Debug, Clone, Copy, Default)]
pub struct PrepareOptions {
    pub max_pixels: Option<u32>,
    pub jpeg_quality: Option<u8>,
}

pub struct PreparedImage {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

pub struct Prepared {
    pub tiled: bool,
    pub resized: bool,
    pub images: Vec<PreparedImage>,
}

impl Prepared {
    fn single(image: PreparedImage, resized: bool) -> Self {
        Self { images: vec![image], resized, tiled: false }
    }

    pub fn total_bytes(&self) -> usize {
        self.images.iter().map(|image| image.bytes.len()).sum()
    }
}

pub fn prepare(bytes: Vec<u8>, mime: &'static str, options: PrepareOptions) -> Prepared {
    let Some(format) = ImageFormat::from_mime_type(mime).filter(|item| matches!(item, ImageFormat::Png | ImageFormat::Jpeg)) else {
        return Prepared::single(PreparedImage { bytes, mime, width: 0, height: 0 }, false);
    };
    let Ok(decoded) = ImageReader::with_format(Cursor::new(&bytes), format).decode() else {
        return Prepared::single(PreparedImage { bytes, mime, width: 0, height: 0 }, false);
    };
    let (width, height) = decoded.dimensions();
    let pixels = width as u64 * height as u64;
    let quality = options.jpeg_quality.unwrap_or(DEFAULT_JPEG_QUALITY).clamp(50, 100);
    let limit = options.max_pixels.filter(|limit| *limit > 0);

    let over_limit = limit.map(|limit| pixels > limit as u64).unwrap_or(false);
    if !over_limit {
        if bytes.len() <= KEEP_ORIGINAL_BELOW_BYTES {
            return Prepared::single(PreparedImage { bytes, mime, width, height }, false);
        }
        return Prepared::single(reencode_or_keep(&decoded, bytes, mime, quality), false);
    }
    let limit = limit.unwrap_or(u32::MAX);
    if pixels as f64 >= limit as f64 * TILE_PIXEL_RATIO {
        return tile(&decoded, limit, quality);
    }
    Prepared::single(shrink_to_limit(&decoded, limit, quality).unwrap_or(PreparedImage { bytes, mime, width, height }), true)
}

pub fn prepare_inline(bytes: Vec<u8>, mime: &'static str, options: PrepareOptions) -> PreparedImage {
    let Some(format) = ImageFormat::from_mime_type(mime).filter(|item| matches!(item, ImageFormat::Png | ImageFormat::Jpeg)) else {
        return PreparedImage { bytes, mime, width: 0, height: 0 };
    };
    let Ok(decoded) = ImageReader::with_format(Cursor::new(&bytes), format).decode() else {
        return PreparedImage { bytes, mime, width: 0, height: 0 };
    };
    let (width, height) = decoded.dimensions();
    let pixels = width as u64 * height as u64;
    let quality = options.jpeg_quality.unwrap_or(DEFAULT_JPEG_QUALITY).clamp(50, 100);
    let Some(limit) = options.max_pixels.filter(|limit| *limit > 0 && pixels > *limit as u64) else {
        if bytes.len() <= KEEP_ORIGINAL_BELOW_BYTES {
            return PreparedImage { bytes, mime, width, height };
        }
        return reencode_or_keep(&decoded, bytes, mime, quality);
    };
    shrink_to_limit(&decoded, limit, quality).unwrap_or(PreparedImage { bytes, mime, width, height })
}

fn shrink_to_limit(image: &DynamicImage, limit: u32, quality: u8) -> Option<PreparedImage> {
    let (width, height) = image.dimensions();
    let pixels = width as u64 * height as u64;
    if pixels <= limit as u64 {
        return encode_jpeg(image, quality);
    }
    let scale = (limit as f64 / pixels as f64).sqrt();
    let target_width = ((width as f64 * scale).floor() as u32).max(1);
    let target_height = ((height as f64 * scale).floor() as u32).max(1);
    let resized = image.resize(target_width, target_height, image::imageops::FilterType::Triangle);
    encode_jpeg(&resized, quality)
}

fn tile(image: &DynamicImage, limit: u32, quality: u8) -> Prepared {
    let (width, height) = image.dimensions();
    let vertical = height >= width;
    let (short_side, long_side) = if vertical { (width, height) } else { (height, width) };
    let step = (limit / short_side.max(1)).max(short_side / 2).max(1);
    let count = ((long_side as f64) / (step as f64)).ceil().max(1.0) as usize;
    let count = count.min(MAX_TILES);
    let step = ((long_side as f64) / (count as f64)).ceil() as u32;
    let mut images = Vec::with_capacity(count);
    for index in 0..count as u32 {
        let start = (index * step).saturating_sub(if index == 0 { 0 } else { TILE_OVERLAP });
        let end = ((index + 1) * step).min(long_side);
        if start >= end {
            break;
        }
        let piece = if vertical { image.crop_imm(0, start, width, end - start) } else { image.crop_imm(start, 0, end - start, height) };
        let Some(encoded) = shrink_to_limit(&piece, limit, quality) else {
            continue;
        };
        images.push(encoded);
    }
    Prepared { images, resized: false, tiled: true }
}

fn reencode_or_keep(image: &DynamicImage, original: Vec<u8>, mime: &'static str, quality: u8) -> PreparedImage {
    let (width, height) = image.dimensions();
    match encode_jpeg(image, quality) {
        Some(encoded) if encoded.bytes.len() < original.len() => encoded,
        _ => PreparedImage { bytes: original, mime, width, height },
    }
}

fn encode_jpeg(image: &DynamicImage, quality: u8) -> Option<PreparedImage> {
    let rgb = image.to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality).encode_image(&rgb).ok()?;
    Some(PreparedImage { bytes: out, mime: "image/jpeg", width: rgb.width(), height: rgb.height() })
}
