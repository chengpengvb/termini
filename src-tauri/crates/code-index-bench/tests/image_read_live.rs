use base64::Engine;
use std::time::Duration;
use serde_json::{json, Value};
use base64::engine::general_purpose::STANDARD;
use code_index_bench::module::ai::tools::common::image::transcript;
use code_index_bench::module::ai::tools::common::image::layout_lines;
use code_index_bench::module::ai::tools::common::image::image_prepare::{self, PrepareOptions, PreparedImage};

const JPEG_QUALITY: u8 = 88;
const MAX_TOKENS: u32 = 8192;
const MAX_PIXELS: u32 = 1_000_000;
const REQUEST_TIMEOUT_SECS: u64 = 600;
const REQUEST_PROMPT: &str = "识别这张图片。";
const OUTPUT_FILE: &str = "image_read_live.md";
const SYSTEM_PROMPT: &str = include_str!("../../termini/ai/resource/prompts/lead/image_read_system.md");

struct Target {
    url: String,
    key: String,
    model: String,
    image: String,
}

fn target() -> Option<Target> {
    let read = |name: &str| std::env::var(name).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
    Some(Target { url: read("TERMINI_VISION_URL")?, key: read("TERMINI_VISION_KEY")?, model: read("TERMINI_VISION_MODEL")?, image: read("TERMINI_VISION_IMAGE")? })
}

fn completions_url(raw: &str) -> String {
    let url = raw.trim().trim_end_matches('/');
    if url.ends_with("/chat/completions") {
        return url.to_string();
    }
    if url.ends_with("/v1") {
        return format!("{}/chat/completions", url);
    }
    format!("{}/v1/chat/completions", url)
}

fn mime_of(path: &str) -> &'static str {
    match path.rsplit('.').next().map(|ext| ext.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/jpeg",
    }
}

fn user_text(index: usize, total: usize) -> String {
    let mut text = REQUEST_PROMPT.to_string();
    if total > 1 {
        text.push_str(&format!("（原图较大，已从上到下切成 {} 个连续片段，这是第 {} 片，请完整输出这一片里从最顶端到最底端的全部内容，顶部被切断的半行也照写。）", total, index + 1));
    }
    text
}

fn request(client: &reqwest::blocking::Client, target: &Target, system_prompt: &str, tile: &PreparedImage, text: String) -> String {
    let data_url = format!("data:{};base64,{}", tile.mime, STANDARD.encode(&tile.bytes));
    let body = json!({
        "model": target.model,
        "stream": false,
        "max_tokens": MAX_TOKENS,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": [ { "type": "text", "text": text }, { "type": "image_url", "image_url": { "url": data_url } } ] }
        ]
    });
    let response = client.post(completions_url(&target.url)).bearer_auth(&target.key).json(&body).send().expect("请求视觉模型");
    let status = response.status();
    let raw = response.text().expect("读取响应");
    assert!(status.is_success(), "status={} body={}", status, raw);
    let value: Value = serde_json::from_str(&raw).expect("解析响应 JSON");
    eprintln!("  usage={}", value["usage"]);
    value["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_string()
}

#[test]
fn reassemble_saved_raw_output() {
    let Some(path) = std::env::var("TERMINI_VISION_RAW_FILE").ok().filter(|value| !value.trim().is_empty()) else {
        eprintln!("skip: 需要 TERMINI_VISION_RAW_FILE 指向 image_read_live 写出的文件");
        return;
    };
    let content = std::fs::read_to_string(&path).expect("读取 raw 文件");
    let raw_section = content.split("===== COMPOSED =====").next().unwrap_or_default();
    let mut texts: Vec<String> = Vec::new();
    for block in raw_section.split("===== tile ").skip(1) {
        let body = block.splitn(2, "raw =====\n").nth(1).unwrap_or_default();
        let text = layout_lines::clean(body);
        eprintln!("tile {} lines={}", texts.len() + 1, text.lines().count());
        texts.push(text);
    }
    let composed = transcript::compose(&layout_lines::stitch(&texts));
    let out_path = std::env::temp_dir().join("image_read_reassembled.md");
    std::fs::write(&out_path, &composed).expect("写输出文件");
    eprintln!("written: {}", out_path.display());
}

#[test]
fn image_read_live() {
    let Some(target) = target() else {
        eprintln!("skip: 需要环境变量 TERMINI_VISION_URL / TERMINI_VISION_KEY / TERMINI_VISION_MODEL / TERMINI_VISION_IMAGE");
        return;
    };
    let system_prompt = std::env::var("TERMINI_VISION_PROMPT_FILE").ok().map(|path| std::fs::read_to_string(path).expect("读取提示词文件")).unwrap_or_else(|| SYSTEM_PROMPT.to_string());
    let bytes = std::fs::read(&target.image).expect("读取图片");
    let prepared = image_prepare::prepare(bytes, mime_of(&target.image), PrepareOptions { max_pixels: Some(MAX_PIXELS), jpeg_quality: Some(JPEG_QUALITY) });
    eprintln!("prepared: tiles={} resized={} tiled={} bytes={}", prepared.images.len(), prepared.resized, prepared.tiled, prepared.total_bytes());

    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)).build().expect("http client");
    let total = prepared.images.len();
    let mut raw_all = String::new();
    let mut texts: Vec<String> = Vec::new();
    for (index, tile) in prepared.images.iter().enumerate() {
        let started = std::time::Instant::now();
        eprintln!("tile {}/{} {}x{}", index + 1, total, tile.width, tile.height);
        let raw = request(&client, &target, &system_prompt, tile, user_text(index, total));
        let text = layout_lines::clean(&raw);
        eprintln!("  elapsed={:.1}s lines={}", started.elapsed().as_secs_f64(), text.lines().count());
        raw_all.push_str(&format!("===== tile {} raw =====\n{}\n", index + 1, raw));
        texts.push(text);
    }
    let composed = transcript::compose(&layout_lines::stitch(&texts));
    let out_path = std::env::temp_dir().join(OUTPUT_FILE);
    std::fs::write(&out_path, format!("{}\n===== COMPOSED =====\n{}\n", raw_all, composed)).expect("写输出文件");
    eprintln!("written: {}", out_path.display());
    assert!(!composed.trim().is_empty(), "识别结果为空");
}
