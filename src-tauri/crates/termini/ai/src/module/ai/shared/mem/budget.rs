use rig::completion::Message;
use rig::message::UserContent;
use crate::module::ai::shared::mem::synthetic;
use crate::module::ai::agent::prompts::LEAD_HOT_MEMORY_TRIMMED;
use crate::service::service_ai_chat_memory::AiChatMemoryService;

pub const FOLD_LOW_PERCENT: usize = 60;
pub const FOLD_HIGH_PERCENT: usize = 80;
pub const FOLD_TAIL_PERCENT: usize = 25;
pub const FOLD_KEEP_PERCENT: usize = 50;
const FOLD_WINDOW_PERCENT: usize = 60;
const FOLD_WINDOW_MIN_TOKENS: usize = 48_000;
const FOLD_WINDOW_MAX_TOKENS: usize = 160_000;

pub fn fold_limit(token_limit: usize) -> usize {
    (token_limit * FOLD_WINDOW_PERCENT / 100).clamp(FOLD_WINDOW_MIN_TOKENS, FOLD_WINDOW_MAX_TOKENS)
}

pub fn estimate(messages: &[Message]) -> usize {
    AiChatMemoryService::estimate_messages_tokens(messages)
}

pub fn estimate_one(message: &Message) -> usize {
    AiChatMemoryService::estimate_messages_tokens(std::slice::from_ref(message))
}

pub fn keep_budget(limit: usize, overhead: usize) -> usize {
    (limit * FOLD_TAIL_PERCENT / 100).max((limit * FOLD_KEEP_PERCENT / 100).saturating_sub(overhead))
}

pub fn pinned_messages(messages: &[Message], addon: &[Message]) -> Vec<Message> {
    let mut out: Vec<Message> = Vec::new();
    for message in messages {
        if !synthetic::is_current_goal_message(message) && !addon.contains(message) {
            continue;
        }
        if !out.contains(message) {
            out.push(message.clone());
        }
    }
    out
}

pub fn compose(pinned: &[Message], body: Vec<Message>) -> Vec<Message> {
    let mut out = Vec::with_capacity(body.len() + pinned.len());
    for message in pinned {
        if !body.contains(message) {
            out.push(message.clone());
        }
    }
    out.extend(body);
    out
}

pub fn split_tail(messages: &[Message], keep_budget: usize) -> Vec<Message> {
    let mut used = 0usize;
    let mut index = messages.len();
    for (position, message) in messages.iter().enumerate().rev() {
        let cost = estimate_one(message);
        if used + cost > keep_budget {
            break;
        }
        used += cost;
        index = position;
    }
    while index < messages.len() {
        if !is_tool_result_only(&messages[index]) {
            break;
        }
        index += 1;
    }
    messages[..index].to_vec()
}

pub fn trim_to_token_limit(messages: Vec<Message>, limit: usize) -> (Vec<Message>, Vec<Message>) {
    let hint = Message::user(LEAD_HOT_MEMORY_TRIMMED);
    let hint_tokens = estimate_one(&hint);

    let mut goal = None;
    let mut body = Vec::with_capacity(messages.len());
    for message in messages {
        if synthetic::is_current_goal_message(&message) {
            goal = Some(message);
        } else {
            body.push(message);
        }
    }

    let goal_tokens = goal.as_ref().map(estimate_one).unwrap_or(0);
    let budget = limit.saturating_sub(hint_tokens).saturating_sub(goal_tokens);

    let mut kept_rev = Vec::new();
    let mut dropped_rev = Vec::new();
    let mut used = 0usize;
    let mut full = false;
    for message in body.into_iter().rev() {
        if full {
            dropped_rev.push(message);
            continue;
        }
        let cost = estimate_one(&message);
        if used + cost > budget && !kept_rev.is_empty() {
            full = true;
            dropped_rev.push(message);
            continue;
        }
        if used + cost > budget && kept_rev.is_empty() {
            full = true;
            kept_rev.push(message);
            continue;
        }
        used += cost;
        kept_rev.push(message);
    }
    kept_rev.reverse();
    dropped_rev.reverse();
    while let Some(first) = kept_rev.first() {
        if !is_tool_result_only(first) {
            break;
        }
        dropped_rev.push(kept_rev.remove(0));
    }

    if let Some(goal) = goal {
        kept_rev.push(goal);
    }
    kept_rev.push(hint);
    (kept_rev, dropped_rev)
}

fn is_tool_result_only(message: &Message) -> bool {
    let Message::User { content } = message else {
        return false;
    };
    content.iter().all(|part| matches!(part, UserContent::ToolResult(_)))
}
