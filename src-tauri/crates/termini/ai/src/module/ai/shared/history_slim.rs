use rig::completion::Message;
use rig::message::{AssistantContent, UserContent};

pub fn slim_history(messages: &[Message], max: usize) -> Vec<Message> {
    let mut out = Vec::with_capacity(messages.len());
    for message in messages {
        match message {
            Message::User { content } => {
                let parts: Vec<UserContent> = content.iter().filter(|part| !matches!(part, UserContent::ToolResult(_))).cloned().collect();
                if !parts.is_empty() {
                    out.push(Message::User { content: parts });
                }
            }
            Message::Assistant { id, content } => {
                let parts: Vec<AssistantContent> = content.iter().filter(|part| !matches!(part, AssistantContent::ToolCall(_))).cloned().collect();
                if !parts.is_empty() {
                    out.push(Message::Assistant { id: id.clone(), content: parts });
                }
            }
            other => out.push(other.clone()),
        }
    }
    out.drain(..out.len().saturating_sub(max));
    out
}
