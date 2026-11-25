use rig::message::ToolCall;
use rig::completion::Message;
use rig::message::UserContent;
use std::collections::HashSet;
use rig::message::AssistantContent;
use crate::module::ai::stream::stream_runner::AiStreamRunner;

impl AiStreamRunner {
    pub(crate) fn sanitize_tool_history(messages: Vec<Message>) -> Vec<Message> {
        let mut index = 0usize;
        let mut out = Vec::with_capacity(messages.len());
        while index < messages.len() {
            match &messages[index] {
                Message::Assistant { id, content } => {
                    let tool_calls: Vec<ToolCall> = content
                        .iter()
                        .filter_map(|part| match part {
                            AssistantContent::ToolCall(call) => Some(call.clone()),
                            _ => None,
                        })
                        .collect();
                    if tool_calls.is_empty() {
                        out.push(messages[index].clone());
                        index += 1;
                        continue;
                    }

                    let call_ids: HashSet<&str> = tool_calls.iter().map(|call| call.wire_call_id()).collect();
                    let mut cursor = index + 1;
                    let mut result_ids: HashSet<&str> = HashSet::new();
                    let mut results: Vec<Message> = Vec::new();
                    while cursor < messages.len() {
                        let Message::User { content: user_content } = &messages[cursor] else {
                            break;
                        };
                        let any_tool = user_content.iter().any(|part| matches!(part, UserContent::ToolResult(_)));
                        if !any_tool {
                            break;
                        }
                        let only_tool = user_content.iter().all(|part| matches!(part, UserContent::ToolResult(_)));
                        let mut parts: Vec<UserContent> = Vec::new();
                        for part in user_content.iter() {
                            let UserContent::ToolResult(result) = part else {
                                parts.push(part.clone());
                                continue;
                            };
                            let wire_id = result.wire_call_id();
                            if !call_ids.contains(wire_id) {
                                continue;
                            }
                            result_ids.insert(wire_id);
                            parts.push(part.clone());
                        }
                        if let Some(message) = Self::user_from_parts(parts) {
                            results.push(message);
                        }
                        cursor += 1;
                        if !only_tool {
                            break;
                        }
                    }

                    let complete = tool_calls.iter().all(|call| result_ids.contains(call.wire_call_id()));
                    if complete {
                        out.push(messages[index].clone());
                        out.extend(results);
                    } else if let Some(kept) = Self::assistant_without_tool_calls(id.clone(), content) {
                        out.push(kept);
                    }
                    index = cursor;
                }
                Message::User { content } => {
                    let only_tool = content.iter().all(|part| matches!(part, UserContent::ToolResult(_)));
                    if only_tool {
                        index += 1;
                        continue;
                    }
                    let non_tool: Vec<UserContent> = content.iter().filter(|part| !matches!(part, UserContent::ToolResult(_))).cloned().collect();
                    if let Some(message) = Self::user_from_parts(non_tool) {
                        out.push(message);
                    }
                    index += 1;
                }
                other => {
                    out.push(other.clone());
                    index += 1;
                }
            }
        }
        out
    }

    fn assistant_without_tool_calls(id: Option<String>, content: &[AssistantContent]) -> Option<Message> {
        let kept: Vec<AssistantContent> = content.iter().filter(|part| !matches!(part, AssistantContent::ToolCall(_))).cloned().collect();
        if kept.is_empty() {
            return None;
        }
        Some(Message::Assistant { id, content: kept })
    }

    fn user_from_parts(parts: Vec<UserContent>) -> Option<Message> {
        if parts.is_empty() {
            return None;
        }
        Some(Message::User { content: parts })
    }
}
