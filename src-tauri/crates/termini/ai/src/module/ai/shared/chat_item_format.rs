use serde_json::Value;
use std::collections::BTreeSet;
use crate::model::ai_chat_item::AiChatItem;
use crate::model::resp::event::resp_event_agent::RespEventAgent;

pub const CHAT_HISTORY_SEARCH_MAX_CHARS: usize = 2000;
pub const CHAT_HISTORY_DEFAULT_USER_LIMIT: usize = 30;
pub const CHAT_HISTORY_SEARCH_CONTEXT_LINES: usize = 2;
pub const CHAT_HISTORY_DEFAULT_SEARCH_LIMIT: usize = 8;
pub const CHAT_HISTORY_TOOL_ARGS_MAX_CHARS: usize = 400;
pub const CHAT_HISTORY_DEFAULT_RECENT_AI_LIMIT: usize = 5;
pub const CHAT_HISTORY_TOOL_RESULT_MAX_CHARS: usize = 800;
pub const CURRENT_GOAL_PREFIX: &str = "[Current Goal]\n";
pub const FOLD_SUMMARY_PREFIX: &str = "[Context Summary]\n以下是本次任务早前经过的折叠摘要，对应原文已移出上下文。需要更早的原文用 chat_history(scope=search) 检索；「操作约定」里的脚本或命令若与你的记忆冲突，一律以约定为准，拿不准就用 select_all_skill 按技能名回查正文，禁止改用通用做法。\n\n";

pub fn intent_prefix(user_prompt: &str, goal: Option<&str>) -> String {
    let mut text = String::new();
    if let Some(goal) = goal.map(str::trim).filter(|value| !value.is_empty()) {
        text.push_str(CURRENT_GOAL_PREFIX);
        text.push_str(goal);
        text.push_str("\n\n");
    }
    text.push_str("[用户]\n");
    text.push_str(user_prompt);
    text.push_str("\n\n");
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatHistoryScope {
    User,
    RecentAi,
    All,
    Search,
}

impl ChatHistoryScope {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "recent_ai" | "ai" => Self::RecentAi,
            "all" => Self::All,
            "search" | "keyword" => Self::Search,
            _ => Self::User,
        }
    }
}

pub fn format_chat_item_line(item: &AiChatItem) -> Option<String> {
    format_line(item, CHAT_HISTORY_TOOL_RESULT_MAX_CHARS)
}

fn format_line(item: &AiChatItem, max_chars: usize) -> Option<String> {
    let ctx = &item.context;
    match item.r#type {
        RespEventAgent::ON_USER => {
            let message = ctx.get("message")?.as_str()?;
            let text = message.trim();
            if text.is_empty() {
                return None;
            }
            Some(format!("[USER] {}", truncate_chars(text, max_chars * 2)))
        }
        RespEventAgent::ON_TEXT_FINAL => {
            let delta = ctx.get("delta")?.as_str()?;
            let text = delta.trim();
            if text.is_empty() {
                return None;
            }
            Some(format!("[ASSISTANT] {}", truncate_chars(text, max_chars)))
        }
        RespEventAgent::ON_TOOL_CALL => {
            let tool_id = read_tool_id(ctx);
            let args = ctx.get("args").map(format_tool_args).unwrap_or_default();
            if args.is_empty() {
                return Some(format!("[TOOL_CALL] {}", tool_id));
            }
            Some(format!("[TOOL_CALL] {} {}", tool_id, args))
        }
        RespEventAgent::ON_TOOL_CALL_DONE => {
            let tool_id = read_tool_id(ctx);
            let result = ctx.get("result").and_then(|v| v.as_str()).unwrap_or("");
            Some(format!("[TOOL_RESULT] {} {}", tool_id, truncate_chars(result.trim(), max_chars)))
        }
        _ => None,
    }
}

fn read_tool_id(ctx: &Value) -> &str {
    ctx.get("toolId").and_then(|v| v.as_str()).map(str::trim).filter(|id| !id.is_empty()).unwrap_or("unknown")
}

pub fn format_tool_args(args: &Value) -> String {
    let Some(map) = args.as_object() else {
        return truncate_chars(args.to_string().trim(), CHAT_HISTORY_TOOL_ARGS_MAX_CHARS);
    };
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in map {
        if key.starts_with("__") || value.is_null() {
            continue;
        }
        let text = match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        parts.push(format!("{}={}", key, text));
    }
    truncate_chars(&parts.join(" "), CHAT_HISTORY_TOOL_ARGS_MAX_CHARS)
}

pub fn is_user_item(item: &AiChatItem) -> bool {
    item.r#type == RespEventAgent::ON_USER
}

pub fn is_recent_ai_item(item: &AiChatItem) -> bool {
    matches!(item.r#type, RespEventAgent::ON_TEXT_FINAL | RespEventAgent::ON_TOOL_CALL | RespEventAgent::ON_TOOL_CALL_DONE)
}

pub fn render_chat_history(items: &[AiChatItem], scope: ChatHistoryScope, limit: usize, query: Option<&str>) -> String {
    let query = query.map(str::trim).filter(|q| !q.is_empty()).map(|q| q.to_ascii_lowercase());
    let mut lines: Vec<String> = Vec::new();

    match scope {
        ChatHistoryScope::User => {
            for item in items.iter().rev() {
                if !is_user_item(item) {
                    continue;
                }
                let Some(line) = format_chat_item_line(item) else {
                    continue;
                };
                if let Some(q) = query.as_deref() {
                    if !line.to_ascii_lowercase().contains(q) {
                        continue;
                    }
                }
                lines.push(line);
                if lines.len() >= limit {
                    break;
                }
            }
            lines.reverse();
        }
        ChatHistoryScope::RecentAi => {
            for item in items.iter().rev() {
                if !is_recent_ai_item(item) {
                    continue;
                }
                let Some(line) = format_chat_item_line(item) else {
                    continue;
                };
                if let Some(q) = query.as_deref() {
                    if !line.to_ascii_lowercase().contains(q) {
                        continue;
                    }
                }
                lines.push(line);
                if lines.len() >= limit {
                    break;
                }
            }
            lines.reverse();
        }
        ChatHistoryScope::All => {
            let mut user_lines: Vec<String> = Vec::new();
            let mut ai_lines: Vec<String> = Vec::new();
            let user_limit = limit.max(1);
            let ai_limit = CHAT_HISTORY_DEFAULT_RECENT_AI_LIMIT;
            for item in items.iter().rev() {
                let Some(line) = format_chat_item_line(item) else {
                    continue;
                };
                if let Some(q) = query.as_deref() {
                    if !line.to_ascii_lowercase().contains(q) {
                        continue;
                    }
                }
                if is_user_item(item) {
                    if user_lines.len() < user_limit {
                        user_lines.push(line);
                    }
                } else if is_recent_ai_item(item) {
                    if ai_lines.len() < ai_limit {
                        ai_lines.push(line);
                    }
                }
                if user_lines.len() >= user_limit && ai_lines.len() >= ai_limit {
                    break;
                }
            }
            user_lines.reverse();
            ai_lines.reverse();
            lines.extend(user_lines);
            if !ai_lines.is_empty() {
                lines.push("--- recent ai ---".to_string());
                lines.extend(ai_lines);
            }
        }
        ChatHistoryScope::Search => {
            let Some(q) = query.as_deref() else {
                return "[chat_history] scope=search requires a query keyword".to_string();
            };
            lines = search_lines(items, limit, q);
        }
    }

    if lines.is_empty() {
        return "[chat_history] (empty)".to_string();
    }
    format!("[chat_history] scope={:?} count={}\n{}", scope, lines.len(), lines.join("\n"))
}

pub fn render_chat_history_page(items: &[AiChatItem], offset: usize, limit: usize) -> String {
    let lines: Vec<String> = items.iter().filter(|item| is_user_item(item) || is_recent_ai_item(item)).filter_map(format_chat_item_line).collect();
    let total = lines.len();
    if offset >= total {
        return format!("[chat_history] offset {} is beyond the end: total {} lines", offset, total);
    }
    let end = (offset + limit).min(total);
    format!("[chat_history] showing lines {}-{} of {} (chronological; pass offset={} for the next page)\n{}", offset + 1, end, total, end, lines[offset..end].join("\n"))
}

fn search_lines(items: &[AiChatItem], limit: usize, query: &str) -> Vec<String> {
    let rendered: Vec<String> = items.iter().filter_map(|item| format_line(item, CHAT_HISTORY_SEARCH_MAX_CHARS)).collect();
    let lowered: Vec<String> = rendered.iter().map(|line| line.to_ascii_lowercase()).collect();
    let tokens: Vec<String> = query.split_whitespace().map(|token| token.to_ascii_lowercase()).collect();
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut hits: Vec<usize> = lowered.iter().enumerate().filter(|(_, line)| tokens.iter().all(|token| line.contains(token))).map(|(index, _)| index).collect();
    if hits.is_empty() && tokens.len() > 1 {
        hits = lowered.iter().enumerate().filter(|(_, line)| tokens.iter().any(|token| line.contains(token))).map(|(index, _)| index).collect();
    }
    if hits.is_empty() {
        return Vec::new();
    }

    let hits = &hits[hits.len().saturating_sub(limit)..];
    let mut keep: BTreeSet<usize> = BTreeSet::new();
    for hit in hits {
        let start = hit.saturating_sub(CHAT_HISTORY_SEARCH_CONTEXT_LINES);
        let end = (hit + CHAT_HISTORY_SEARCH_CONTEXT_LINES).min(rendered.len().saturating_sub(1));
        for index in start..=end {
            keep.insert(index);
        }
    }

    let mut lines: Vec<String> = Vec::with_capacity(keep.len());
    let mut previous: Option<usize> = None;
    for index in keep {
        if previous.is_some_and(|last| index > last + 1) {
            lines.push("---".to_string());
        }
        lines.push(rendered[index].clone());
        previous = Some(index);
    }
    lines
}

pub fn truncate_chars(input: &str, max_chars: usize) -> String {
    let count = input.chars().count();
    if count <= max_chars {
        return input.to_string();
    }
    let truncated: String = input.chars().take(max_chars).collect();
    format!("{}...(truncated)", truncated)
}
