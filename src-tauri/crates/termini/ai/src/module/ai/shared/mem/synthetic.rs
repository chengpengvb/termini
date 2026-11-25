use rig::completion::Message;
use rig::message::UserContent;
use crate::module::ai::agent::prompts::LEAD_HOT_MEMORY_TRIMMED;
use crate::service::service_ai_chat_memory::AiChatMemoryService;
use crate::module::ai::shared::chat_item_format::{CURRENT_GOAL_PREFIX, FOLD_SUMMARY_PREFIX};

pub fn is_current_goal_message(message: &Message) -> bool {
    user_text_matches(message, |text| text.starts_with(CURRENT_GOAL_PREFIX))
}

pub fn is_synthetic_text(text: &str) -> bool {
    text.starts_with(CURRENT_GOAL_PREFIX) || text.starts_with(FOLD_SUMMARY_PREFIX) || text == LEAD_HOT_MEMORY_TRIMMED
}

pub fn is_synthetic_message(message: &Message) -> bool {
    user_text_matches(message, is_synthetic_text)
}

pub fn strip_synthetic(messages: Vec<Message>) -> Vec<Message> {
    messages.into_iter().filter(|message| !is_synthetic_message(message)).collect()
}

pub fn strip_images(messages: Vec<Message>) -> Vec<Message> {
    messages
        .into_iter()
        .filter_map(|message| {
            let Message::User { content } = message else {
                return Some(message);
            };
            let kept: Vec<UserContent> = content.into_iter().filter(|part| !matches!(part, UserContent::Image(_))).collect();
            if kept.is_empty() {
                None
            } else {
                Some(Message::User { content: kept })
            }
        })
        .collect()
}

pub fn with_summary(summary: &str, messages: Vec<Message>) -> Vec<Message> {
    if summary.is_empty() {
        return messages;
    }
    let mut out = Vec::with_capacity(messages.len() + 1);
    out.push(summary_message(summary));
    out.extend(messages);
    out
}

pub fn summary_message(summary: &str) -> Message {
    Message::user(format!("{FOLD_SUMMARY_PREFIX}{summary}"))
}

pub fn summary_tokens(summary: &str) -> usize {
    if summary.is_empty() {
        return 0;
    }
    AiChatMemoryService::estimate_messages_tokens(std::slice::from_ref(&summary_message(summary)))
}

fn user_text_matches(message: &Message, predicate: impl Fn(&str) -> bool) -> bool {
    match message {
        Message::User { content } => content.iter().any(|part| match part {
            UserContent::Text(text) => predicate(&text.text),
            _ => false,
        }),
        _ => false,
    }
}
