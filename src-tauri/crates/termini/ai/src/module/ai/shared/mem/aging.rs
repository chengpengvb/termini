use rig::completion::Message;
use rig::message::{ToolResultContent, UserContent};

pub const AGE_KEEP_RECENT_TURNS: usize = 2;

const AGE_MIN_CHARS: usize = 200;

pub fn age_tool_results(messages: &mut [Message], keep_turns: usize) -> usize {
    let boundary = keep_boundary(messages, keep_turns);
    let mut aged = 0usize;
    for message in messages.iter_mut().take(boundary) {
        let Message::User { content } = message else {
            continue;
        };
        if !content.iter().any(|part| matches!(part, UserContent::ToolResult(_))) {
            continue;
        }
        let parts: Vec<UserContent> = content.iter().map(|part| age_part(part, &mut aged)).collect();
        *content = parts;
    }
    aged
}

fn age_part(part: &UserContent, aged: &mut usize) -> UserContent {
    let UserContent::ToolResult(result) = part else {
        return part.clone();
    };
    let total: usize = result
        .content
        .iter()
        .map(|item| match item {
            ToolResultContent::Text(text) => text.text.chars().count(),
            ToolResultContent::Image(_) => AGE_MIN_CHARS + 1,
            ToolResultContent::Json { value } => value.to_string().chars().count(),
        })
        .sum();
    if total <= AGE_MIN_CHARS {
        return part.clone();
    }
    *aged += 1;
    let mut next = result.clone();
    next.content = vec![ToolResultContent::text(format!("[结果已老化: 原 {} 字符已移出上下文, 如仍需该数据请重新调用工具获取]", total))];
    UserContent::ToolResult(next)
}

fn keep_boundary(messages: &[Message], keep_turns: usize) -> usize {
    let mut seen = 0usize;
    for (index, message) in messages.iter().enumerate().rev() {
        if is_real_user_text(message) {
            seen += 1;
            if seen >= keep_turns {
                return index;
            }
        }
    }
    0
}

fn is_real_user_text(message: &Message) -> bool {
    let Message::User { content } = message else {
        return false;
    };
    if content.iter().any(|part| matches!(part, UserContent::ToolResult(_))) {
        return false;
    }
    content.iter().any(|part| match part {
        UserContent::Text(text) => !text.text.starts_with('['),
        _ => false,
    })
}
