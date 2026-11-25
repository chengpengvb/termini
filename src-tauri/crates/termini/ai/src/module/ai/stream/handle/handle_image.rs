use serde_json::Value;
use summer::tracing::{info, warn};
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::module::ai::shared::chat_file_store::CHAT_IMAGES_DIR;
use crate::module::ai::shared::generated_image::{image_url_of, save_generated_image, GeneratedImage};

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn handle_stream_image(&self, context: &AiRunContext, value: &Value) -> Option<GeneratedImage> {
        let url = image_url_of(value)?;
        let dir = match self.chat_file_store.ensure_dir_for_req(&context.req_id, CHAT_IMAGES_DIR).await {
            Ok(dir) => dir,
            Err(error) => {
                warn!("[ai] 解析对话图片目录失败 req_id={} run_id={} error={:#}", context.req_id, context.run_id, error);
                return None;
            }
        };
        let saved = match save_generated_image(&dir, &url).await {
            Ok(saved) => saved,
            Err(error) => {
                warn!("[ai] 保存模型生成的图片失败 req_id={} run_id={} error={:#}", context.req_id, context.run_id, error);
                return None;
            }
        };
        info!("[ai] 模型生成图片已保存 req_id={} run_id={} path={} media_type={}", context.req_id, context.run_id, saved.path, saved.media_type);
        self.generated_image_push.push(context, &saved);
        Some(saved)
    }
}
