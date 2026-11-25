use rig::completion::Message;
use rig::message::{AssistantContent, UserContent};
use crate::module::ai::shared::chat_item_format::{format_tool_args, truncate_chars};

const TRANSCRIPT_MAX_CHARS: usize = 16000;
const TRANSCRIPT_LINE_MAX_CHARS: usize = 600;
const TRANSCRIPT_TOOL_RESULT_MAX_CHARS: usize = 150;

pub fn build_transcript(dropped: &[Message]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for message in dropped {
        match message {
            Message::User { content } => {
                for part in content.iter() {
                    match part {
                        UserContent::Text(text) => lines.push(user_line(&text.text)),
                        UserContent::ToolResult(result) => lines.push(line_capped("工具结果", &result_text(result), TRANSCRIPT_TOOL_RESULT_MAX_CHARS)),
                        _ => {}
                    }
                }
            }
            Message::Assistant { content, .. } => {
                for part in content.iter() {
                    match part {
                        AssistantContent::Text(text) => lines.push(line_capped("助手", &text.text, TRANSCRIPT_LINE_MAX_CHARS)),
                        AssistantContent::ToolCall(tool_call) => lines.push(line_capped("工具调用", &tool_call_text(tool_call), TRANSCRIPT_LINE_MAX_CHARS)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    let text = lines.into_iter().filter(|line| !line.is_empty()).collect::<Vec<String>>().join("\n");
    truncate_tail(&text, TRANSCRIPT_MAX_CHARS)
}

pub fn tool_call_text(tool_call: &rig::message::ToolCall) -> String {
    format!("{} {}", tool_call.function.name, format_tool_args(&tool_call.function.arguments))
}

pub fn result_text(result: &rig::message::ToolResult) -> String {
    result.content.iter().filter_map(|item| item.as_text()).collect::<Vec<_>>().join("\n")
}

pub fn truncate_tail(input: &str, max_chars: usize) -> String {
    let count = input.chars().count();
    if count <= max_chars {
        return input.to_string();
    }
    input.chars().skip(count - max_chars).collect()
}

fn line_capped(role: &str, text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    format!("[{}] {}", role, truncate_chars(text, max_chars))
}

fn user_line(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    format!("[用户] {}", text)
}
