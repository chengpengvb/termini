use base64::Engine;
use std::path::Path;
use serde_json::Value;
use crate::util::id_factory::uuid;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD;

const BASE64_MARK: &str = ";base64,";
const DATA_URL_PREFIX: &str = "data:";
const REMOTE_IMAGE_MAX_BYTES: usize = 20 * 1024 * 1024;

pub struct GeneratedImage {
    pub path: String,
    pub media_type: String,
}

pub fn image_url_of(value: &Value) -> Option<String> {
    let image_url = value.get("image_url")?;
    if let Some(url) = image_url.as_str() {
        return Some(url.to_string());
    }
    image_url.get("url")?.as_str().map(str::to_string)
}

pub async fn save_generated_image(dir: &Path, url: &str) -> Result<GeneratedImage> {
    let (media_type, bytes) = decode_image_source(url).await?;
    save_image_bytes(dir, media_type, bytes).await
}

pub async fn save_image_base64(dir: &Path, payload: &str) -> Result<GeneratedImage> {
    let bytes = STANDARD.decode(payload.trim()).context("decode image base64")?;
    let format = image::guess_format(&bytes).context("failed to detect image format")?;
    save_image_bytes(dir, format.to_mime_type().to_string(), bytes).await
}

async fn save_image_bytes(dir: &Path, media_type: String, bytes: Vec<u8>) -> Result<GeneratedImage> {
    let extension = image_extension(&media_type).ok_or_else(|| anyhow!("unsupported image type: {}", media_type))?;
    let path = dir.join(format!("{}.{}", uuid(), extension));
    tokio::fs::write(&path, bytes).await.with_context(|| format!("write {}", path.display()))?;
    Ok(GeneratedImage { path: path.display().to_string(), media_type })
}

async fn decode_image_source(url: &str) -> Result<(String, Vec<u8>)> {
    if let Some(rest) = url.strip_prefix(DATA_URL_PREFIX) {
        let (media_type, payload) = rest.split_once(BASE64_MARK).ok_or_else(|| anyhow!("data url is not base64 encoded"))?;
        let bytes = STANDARD.decode(payload.trim()).context("decode image base64")?;
        return Ok((media_type.trim().to_string(), bytes));
    }
    let response = reqwest::get(url).await.with_context(|| format!("download {}", url))?;
    let header = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).unwrap_or_default();
    let media_type = header.split(';').next().unwrap_or(header).trim().to_string();
    let bytes = response.bytes().await.context("read image body")?;
    if bytes.len() > REMOTE_IMAGE_MAX_BYTES {
        return Err(anyhow!("image too large: {} bytes", bytes.len()));
    }
    Ok((media_type, bytes.to_vec()))
}

fn image_extension(media_type: &str) -> Option<&'static str> {
    match media_type {
        "image/png" => Some("png"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        _ => None,
    }
}
