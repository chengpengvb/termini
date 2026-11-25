use rig::completion::Message;
use rig::message::UserContent;
use crate::module::ai::shared::MemoryCompactor;
use crate::module::ai::agent::prompts::LEAD_HOT_MEMORY_TRIMMED;
use crate::service::service_ai_chat_memory::AiChatMemoryService;
use crate::module::ai::shared::chat_item_format::CURRENT_GOAL_PREFIX;

fn user(text: &str) -> Message {
    Message::user(text)
}

fn goal(text: &str) -> Message {
    Message::user(format!("{CURRENT_GOAL_PREFIX}{text}"))
}

fn user_texts(messages: &[Message]) -> Vec<String> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::User { content } => content.iter().find_map(|part| match part {
                UserContent::Text(text) => Some(text.text.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect()
}

#[test]
fn trim_keeps_recent_and_goal() {
    let messages = vec![user("old-1"), user("old-2"), user("recent"), goal("ship FakeNOS")];
    let limit = AiChatMemoryService::estimate_messages_tokens(&[user("recent"), goal("ship FakeNOS"), user(LEAD_HOT_MEMORY_TRIMMED)]);
    let (next, dropped) = MemoryCompactor::trim_to_token_limit(messages, limit);
    let texts = user_texts(&next);
    assert_eq!(dropped.len(), 2);
    assert!(texts.iter().any(|text| text == "recent"));
    assert!(texts.iter().any(|text| text.starts_with(CURRENT_GOAL_PREFIX)));
    assert!(texts.iter().any(|text| text.starts_with("[Hot Memory Trimmed]")));
    assert!(!texts.iter().any(|text| text == "old-1"));
}

#[test]
fn trim_reports_dropped_messages() {
    let messages = vec![user("old-1"), user("old-2"), user("recent")];
    let limit = AiChatMemoryService::estimate_messages_tokens(&[user("recent"), user(LEAD_HOT_MEMORY_TRIMMED)]);
    let (_, dropped) = MemoryCompactor::trim_to_token_limit(messages, limit);
    assert_eq!(user_texts(&dropped), vec!["old-1".to_string(), "old-2".to_string()]);
}
