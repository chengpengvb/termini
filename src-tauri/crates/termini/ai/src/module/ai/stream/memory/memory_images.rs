use rig::completion::Message;
use rig::message::UserContent;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_runner::AiStreamRunner;

const IMAGE_DROPPED_TEXT: &str = "[图片已忽略：当前模型不支持图片]";

impl AiStreamRunner {
    pub(in crate::module::ai::stream) fn memory_has_images(context: &AiRunContext) -> bool {
        if !context.prompt_images.lock().is_empty() {
            return true;
        }
        context.memory.lock().iter().any(|message| match message {
            Message::User { content } => content.iter().any(|part| matches!(part, UserContent::Image(_))),
            _ => false,
        })
    }

    pub(in crate::module::ai::stream) fn drop_memory_images(context: &AiRunContext) -> usize {
        let mut dropped = context.drop_prompt_images();
        let mut memory = context.memory.lock();
        let stripped = std::mem::take(&mut *memory)
            .into_iter()
            .map(|message| {
                let Message::User { content } = message else {
                    return message;
                };
                let kept: Vec<UserContent> = content
                    .into_iter()
                    .filter(|part| {
                        if !matches!(part, UserContent::Image(_)) {
                            return true;
                        }
                        dropped += 1;
                        false
                    })
                    .collect();
                if kept.is_empty() {
                    Message::user(IMAGE_DROPPED_TEXT)
                } else {
                    Message::User { content: kept }
                }
            })
            .collect();
        *memory = stripped;
        dropped
    }
}
