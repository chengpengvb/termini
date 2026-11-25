use rig::completion::Message;
use crate::module::ai::shared::mem::synthetic;
use rig::message::{AssistantContent, UserContent};
use crate::module::ai::agent::agent_fold::FOLD_MAX_CHARS;
use crate::module::ai::shared::chat_item_format::truncate_chars;
use crate::module::ai::shared::mem::transcript::{tool_call_text, truncate_tail};

const CALL_MAX_CHARS: usize = 80;
const USER_MAX_CHARS: usize = 200;
const PREVIOUS_PERCENT: usize = 60;
const ASSISTANT_MAX_CHARS: usize = 160;
const RECENT_HEADER: &str = "## 近期经过（自动摘录, 摘要模型不可用）";

pub fn mechanical_summary(previous: &str, to_fold: &[Message]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for message in to_fold {
        match message {
            Message::User { content } => {
                for part in content.iter() {
                    if let UserContent::Text(text) = part {
                        let text = text.text.trim();
                        if !text.is_empty() && !synthetic::is_synthetic_text(text) {
                            lines.push(format!("- 用户: {}", truncate_chars(text, USER_MAX_CHARS)));
                        }
                    }
                }
            }
            Message::Assistant { content, .. } => {
                let mut calls: Vec<String> = Vec::new();
                for part in content.iter() {
                    match part {
                        AssistantContent::Text(text) => {
                            let text = text.text.trim();
                            if !text.is_empty() {
                                lines.push(format!("- 助手: {}", truncate_chars(text, ASSISTANT_MAX_CHARS)));
                            }
                        }
                        AssistantContent::ToolCall(tool_call) => calls.push(truncate_chars(&tool_call_text(tool_call), CALL_MAX_CHARS)),
                        _ => {}
                    }
                }
                if !calls.is_empty() {
                    lines.push(format!("- 调用: {}", calls.join("; ")));
                }
            }
            _ => {}
        }
    }
    let head_budget = if previous.is_empty() { 0 } else { FOLD_MAX_CHARS * PREVIOUS_PERCENT / 100 };
    let head = truncate_chars(previous.trim(), head_budget);
    let tail_budget = FOLD_MAX_CHARS.saturating_sub(head.chars().count()).saturating_sub(RECENT_HEADER.chars().count() + 2);
    let recent = truncate_tail(&lines.join("\n"), tail_budget);
    if head.is_empty() {
        format!("{}\n{}", RECENT_HEADER, recent)
    } else {
        format!("{}\n\n{}\n{}", head, RECENT_HEADER, recent)
    }
}
