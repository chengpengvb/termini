use base64::Engine;
use std::path::PathBuf;
use anyhow::{Context, Result};
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::model::ai_provide::AiProvide;
use crate::module::ai::agent_chat::AgentChat;
use base64::engine::general_purpose::STANDARD;
use crate::model::req::req_chat_context::ReqChatImage;
use rig::message::{ImageMediaType, MimeType, UserContent};
use crate::module::ai::tools::common::image::image_prepare;
use crate::module::ai::shared::chat_file_store::{CHAT_IMAGES_DIR, CHAT_TEXTS_DIR};

const INLINE_IMAGE_MAX: usize = 4;
const MAX_CHAT_IMAGE_COUNT: usize = 25;
const MAX_INLINE_PROMPT_BYTES: usize = 32 * 1024;
const MAX_CHAT_IMAGE_DATA_BYTES: usize = 10 * 1024 * 1024;

impl AgentChat {
    pub(super) async fn build_inline_images(&self, ai_provide: &AiProvide, image_paths: &[String]) -> Result<Vec<UserContent>> {
        if !ai_provide.vision || image_paths.is_empty() {
            return Ok(Vec::new());
        }
        let options = self.vision_transcriber.prepare_options().await?;
        let mut images = Vec::new();
        for path in image_paths.iter().take(INLINE_IMAGE_MAX) {
            let Some(mime) = Self::mime_of_path(path) else {
                continue;
            };
            let bytes = match tokio::fs::read(path).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    warn!("[ai] 读取内联图片失败 path={} error={}", path, err);
                    continue;
                }
            };
            let source_len = bytes.len();
            let prepared = match tokio::task::spawn_blocking(move || image_prepare::prepare_inline(bytes, mime, options)).await {
                Ok(prepared) => prepared,
                Err(err) => {
                    warn!("[ai] 内联图片预处理失败 path={} error={}", path, err);
                    continue;
                }
            };
            info!("[ai] 图片内联给视觉模型 path={} {}→{} bytes {}x{}", path, source_len, prepared.bytes.len(), prepared.width, prepared.height);
            images.push(UserContent::image_base64(STANDARD.encode(&prepared.bytes), ImageMediaType::from_mime_type(prepared.mime), None));
        }
        Ok(images)
    }

    pub(super) async fn resolve_user_prompt(&self, chat_id: &str, prompt: String) -> Result<String> {
        if prompt.len() <= MAX_INLINE_PROMPT_BYTES {
            return Ok(prompt);
        }
        let path = self.save_text(chat_id, &prompt).await?;
        Ok(format!("@[localpath:{}]", path))
    }

    pub(super) async fn save_images(&self, chat_id: &str, images: &[ReqChatImage]) -> Result<Vec<String>> {
        let mut paths = Vec::new();
        if images.is_empty() {
            return Ok(paths);
        }
        let dir = self.chat_file_store.ensure_dir(chat_id, CHAT_IMAGES_DIR).await?;
        for image in images {
            if image.data.len() > MAX_CHAT_IMAGE_DATA_BYTES {
                warn!("[ai] 丢弃超大图片 data_len={} 上限={}", image.data.len(), MAX_CHAT_IMAGE_DATA_BYTES);
                continue;
            }
            let Some(ext) = Self::image_extension(&image.media_type) else {
                warn!("[ai] 丢弃不支持的图片类型 media_type={}", image.media_type);
                continue;
            };
            let bytes = STANDARD.decode(image.data.as_bytes()).context("decode image base64")?;
            let path = dir.join(format!("{}.{}", uuid(), ext));
            tokio::fs::write(&path, bytes).await.with_context(|| format!("write {}", path.display()))?;
            paths.push(path.display().to_string());
            if paths.len() >= MAX_CHAT_IMAGE_COUNT {
                break;
            }
        }
        Ok(paths)
    }

    fn mime_of_path(path: &str) -> Option<&'static str> {
        let ext = PathBuf::from(path).extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "webp" => Some("image/webp"),
            "gif" => Some("image/gif"),
            _ => None,
        }
    }

    async fn save_text(&self, chat_id: &str, prompt: &str) -> Result<String> {
        let dir = self.chat_file_store.ensure_dir(chat_id, CHAT_TEXTS_DIR).await?;
        let path = dir.join(format!("{}.txt", uuid()));
        tokio::fs::write(&path, prompt.as_bytes()).await.with_context(|| format!("write {}", path.display()))?;
        Ok(path.display().to_string())
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

    pub(super) fn build_llm_prompt(prompt: &str, image_paths: &[String], inline_count: usize) -> String {
        if image_paths.is_empty() {
            return prompt.to_string();
        }
        let total = image_paths.len();
        let list = image_paths.iter().enumerate().map(|(index, path)| Self::image_line(index, path, inline_count)).collect::<Vec<String>>().join("\n");
        if inline_count == 0 {
            return format!("{}\n\n[Attached Images]\n用户随本条消息上传了 {} 张图片，已保存为本地文件。需要了解图片内容时，用 image_read 工具按路径读取：\n{}", prompt, total, list);
        }
        let rest = total.saturating_sub(inline_count);
        let overflow = if rest == 0 { String::new() } else { format!("其余 {} 张未随消息附带，需要了解内容时用 image_read 按路径读取。", rest) };
        let head = format!("用户随本条消息上传了 {} 张图片，已保存为本地文件；其中前 {} 张已直接附在本条消息中，你可以直接查看。{}", total, inline_count, overflow);
        let guide = "直接看图适合理解画面、回答问题、粗读内容；若用户要求逐行精确转写、保持原始排版/换行/表格/试卷格式，或图片文字密集、长图看不清，请改用 image_read 按路径读取（它按版面 1:1 还原并落盘），不要凭直接观察硬抄。";
        format!("{}\n\n[Attached Images]\n{}\n{}\n{}", prompt, head, guide, list)
    }

    fn image_line(index: usize, path: &str, inline_count: usize) -> String {
        if index < inline_count {
            return format!("- [{}] {} (已附在消息中)", index + 1, path);
        }
        format!("- [{}] {}", index + 1, path)
    }
}
