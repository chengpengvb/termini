use serde_json::Value;
use rig::completion::Message;
use std::collections::{HashMap, HashSet};
use crate::module::ai::tools::base::INTERNAL_CALL_ID_KEY;
use rig::message::{AssistantContent, ToolResult, ToolResultContent, UserContent};

const SUPERSEDED_TEXT: &str = "[结果已被同参数的后续调用取代: 原文已移出上下文, 看最新那次的结果即可]";

pub fn prune_superseded(messages: &mut [Message], tools: &HashSet<String>) -> usize {
    if tools.is_empty() {
        return 0;
    }
    let keys = call_keys(messages, tools);
    if keys.is_empty() {
        return 0;
    }
    let newest = newest_by_key(messages, &keys);
    let mut pruned = 0usize;
    for (index, message) in messages.iter_mut().enumerate() {
        let Message::User { content } = message else {
            continue;
        };
        let next: Vec<UserContent> = content.iter().map(|part| prune_part(part, index, &keys, &newest, &mut pruned)).collect();
        *content = next;
    }
    pruned
}

fn call_keys(messages: &[Message], tools: &HashSet<String>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for message in messages {
        let Message::Assistant { content, .. } = message else {
            continue;
        };
        for part in content.iter() {
            let AssistantContent::ToolCall(call) = part else {
                continue;
            };
            if !tools.contains(&call.function.name) {
                continue;
            }
            out.insert(call.wire_call_id().to_string(), call_key(&call.function.name, &call.function.arguments));
        }
    }
    out
}

fn call_key(name: &str, arguments: &Value) -> String {
    let mut value = arguments.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove(INTERNAL_CALL_ID_KEY);
    }
    format!("{name}:{value}")
}

fn newest_by_key(messages: &[Message], keys: &HashMap<String, String>) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for (index, message) in messages.iter().enumerate() {
        let Message::User { content } = message else {
            continue;
        };
        for part in content.iter() {
            let UserContent::ToolResult(result) = part else {
                continue;
            };
            let Some(key) = keys.get(result.wire_call_id()) else {
                continue;
            };
            out.insert(key.clone(), index);
        }
    }
    out
}

fn prune_part(part: &UserContent, index: usize, keys: &HashMap<String, String>, newest: &HashMap<String, usize>, pruned: &mut usize) -> UserContent {
    let UserContent::ToolResult(result) = part else {
        return part.clone();
    };
    let Some(key) = keys.get(result.wire_call_id()) else {
        return part.clone();
    };
    let Some(newest) = newest.get(key) else {
        return part.clone();
    };
    if *newest <= index {
        return part.clone();
    }
    if is_pruned(result) {
        return part.clone();
    }
    *pruned += 1;
    let mut next = result.clone();
    next.content = vec![ToolResultContent::text(SUPERSEDED_TEXT)];
    UserContent::ToolResult(next)
}

fn is_pruned(result: &ToolResult) -> bool {
    let Some(ToolResultContent::Text(text)) = result.content.first() else {
        return false;
    };
    result.content.len() == 1 && text.text == SUPERSEDED_TEXT
}
