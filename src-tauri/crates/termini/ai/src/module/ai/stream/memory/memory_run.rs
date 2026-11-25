use std::sync::Arc;
use summer::tracing::warn;
use rig::message::ToolCall;
use rig::completion::Message;
use rig::message::AssistantContent;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::mem::fold_plan::FoldState;
use crate::module::ai::shared::mem::synthetic::with_summary;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::module::ai::shared::generated_image::GeneratedImage;

const GENERATED_IMAGES_PREFIX: &str = "[Generated Images]";

impl AiStreamRunner {
    pub(in crate::module::ai::stream) fn note_generated_images(context: &AiRunContext, images: &[GeneratedImage]) {
        if images.is_empty() {
            return;
        }
        let list = images.iter().map(|image| format!("- {}", image.path)).collect::<Vec<String>>().join("\n");
        let text = format!("{}\n上一轮模型生成的图片已保存为本地文件，用户已在界面看到，需要引用时直接给路径：\n{}", GENERATED_IMAGES_PREFIX, list);
        context.memory.lock().push(Message::user(text));
    }

    pub(in crate::module::ai::stream) fn without_addon_messages(messages: Vec<Message>, addon_messages: &[Message]) -> Vec<Message> {
        if addon_messages.is_empty() {
            return messages;
        }
        messages.into_iter().filter(|message| !addon_messages.iter().any(|addon| addon == message)).collect()
    }

    pub(in crate::module::ai::stream) fn merge_run_messages(history: &[Message], run_messages: Vec<Message>, addon_messages: &[Message]) -> Vec<Message> {
        let mut merged = Vec::with_capacity(history.len() + run_messages.len());
        merged.extend_from_slice(history);
        merged.extend(run_messages);
        Self::sanitize_tool_history(Self::without_addon_messages(merged, addon_messages))
    }

    pub(in crate::module::ai::stream) fn push_pending_text(run_messages: &mut Vec<Message>, pending_text: &mut String) {
        let text = std::mem::take(pending_text);
        if text.trim().is_empty() {
            return;
        }
        run_messages.push(Message::assistant(text));
    }

    pub(in crate::module::ai::stream) fn push_tool_call(run_messages: &mut Vec<Message>, call: ToolCall) {
        let part = AssistantContent::ToolCall(call);
        if let Some(Message::Assistant { content, .. }) = run_messages.last_mut() {
            if content.iter().all(|item| matches!(item, AssistantContent::ToolCall(_))) {
                content.push(part);
                return;
            }
        }
        run_messages.push(Message::Assistant { id: None, content: vec![part] });
    }

    pub(in crate::module::ai::stream) fn keep_run_messages(context: &AiRunContext, history: &[Message], run_messages: &[Message]) {
        let kept = Self::merge_run_messages(history, run_messages.to_vec(), &context.addon_messages);
        let kept_len = kept.len();
        *context.memory.lock() = kept;
        warn!("[ai] 流出错，已回写记忆 run_id={} memory_msgs={}", context.run_id, kept_len);
    }

    pub(in crate::module::ai::stream) async fn folded_history(history: &[Message], fold_state: &Arc<tokio::sync::Mutex<FoldState>>) -> Vec<Message> {
        let guard = fold_state.lock().await;
        if guard.folded == 0 {
            return history.to_vec();
        }
        let kept = history[guard.folded.min(history.len())..].to_vec();
        with_summary(&guard.summary, kept)
    }
}
