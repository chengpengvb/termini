use std::path::Path;
use serde_json::Value;
use anyhow::{anyhow, Result};
use rig::completion::Message;
use std::collections::HashMap;
use crate::service::service_base::BaseService;
use crate::service::service_group::GroupService;
use crate::service::service_device::DeviceService;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::module::ai::mcp::dto::mcp_prompt_snapshot::McpPromptSnapshot;
use rmcp::model::{JsonObject, PromptMessage, PromptMessageContent, PromptMessageRole, ResourceContents};
use crate::model::req::req_chat_reference::{ReqRefDevice, ReqRefGroup, ReqRefKind, ReqRefMcpServer, ReqRefPath, ReqRefTargets, ReqRefToken, REQ_REF_DIR, REQ_REF_FILE, REQ_REF_PREFIX};

pub async fn build_referenced_targets_message(prompt: &str, device_service: &DeviceService, group_service: &GroupService) -> Result<Option<String>> {
    let tokens = parse_ref_tokens(prompt);
    let target_tokens: Vec<ReqRefToken> = tokens.into_iter().filter(|token| matches!(token.kind, ReqRefKind::Device | ReqRefKind::Group | ReqRefKind::LocalPath)).collect();
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let all_devices = device_service.select_all_with_config().await?;
    let mut device_by_id: HashMap<String, ReqRefDevice> = HashMap::new();
    let mut devices_by_group: HashMap<String, Vec<String>> = HashMap::new();
    for item in all_devices {
        let device = item.device();
        let profile = device.profile.as_deref().map(str::trim).filter(|text| !text.is_empty()).map(str::to_string);
        let referenced = ReqRefDevice { device_id: device.id.clone(), name: device.name.clone(), host: device.host.clone(), os_type: device.os_type, profile };
        if let Some(group_id) = item.group_id() {
            devices_by_group.entry(group_id.to_string()).or_default().push(device.id.clone());
        }
        device_by_id.insert(device.id.clone(), referenced);
    }

    let all_groups = group_service.find_all().await?;
    let mut group_by_id: HashMap<String, String> = HashMap::new();
    for group in all_groups {
        let group = group.read();
        group_by_id.insert(group.id.clone(), group.name.clone());
    }

    let mut devices: Vec<ReqRefDevice> = Vec::new();
    let mut groups: Vec<ReqRefGroup> = Vec::new();
    let mut local_paths: Vec<ReqRefPath> = Vec::new();

    for token in target_tokens {
        match token.kind {
            ReqRefKind::Device => {
                let Some(device) = device_by_id.get(&token.id) else {
                    return Err(anyhow!("referenced device not found: {}", token.id));
                };
                devices.push(device.clone());
            }
            ReqRefKind::Group => {
                let Some(name) = group_by_id.get(&token.id) else {
                    return Err(anyhow!("referenced group not found: {}", token.id));
                };
                let device_ids = devices_by_group.get(&token.id).cloned().unwrap_or_default();
                for device_id in &device_ids {
                    let Some(device) = device_by_id.get(device_id) else {
                        continue;
                    };
                    devices.push(device.clone());
                }
                groups.push(ReqRefGroup { group_id: token.id, name: name.clone(), device_ids });
            }
            ReqRefKind::LocalPath => {
                local_paths.push(resolve_local_path(&token.id)?);
            }
            ReqRefKind::Skill | ReqRefKind::McpPrompt | ReqRefKind::McpTool | ReqRefKind::McpServer => {}
        }
    }

    if devices.is_empty() && groups.is_empty() && local_paths.is_empty() {
        return Ok(None);
    }

    let payload = ReqRefTargets { groups, devices, local_paths };
    let text = serde_json::to_string(&payload)?;
    Ok(Some(format!("[Referenced Chat Targets]{text}")))
}

fn resolve_local_path(path: &str) -> Result<ReqRefPath> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("referenced local path is empty"));
    }
    let path_buf = Path::new(trimmed);
    if !path_buf.is_absolute() {
        return Err(anyhow!("referenced local path must be absolute: {}", trimmed));
    }
    if !path_buf.exists() {
        return Err(anyhow!("referenced local path not found: {}", trimmed));
    }
    let entry_kind = if path_buf.is_dir() {
        REQ_REF_DIR
    } else if path_buf.is_file() {
        REQ_REF_FILE
    } else {
        return Err(anyhow!("referenced local path is neither file nor directory: {}", trimmed));
    };
    let name = resolve_local_path_name(trimmed);
    Ok(ReqRefPath { path: trimmed.to_string(), name, entry_kind })
}

fn resolve_local_path_name(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        return path.to_string();
    }
    let slash = trimmed.rfind('/');
    let backslash = trimmed.rfind('\\');
    let index = match (slash, backslash) {
        (Some(a), Some(b)) => a.max(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => return trimmed.to_string(),
    };
    trimmed[index + 1..].to_string()
}

fn is_adjacent_gap(gap: &str) -> bool {
    gap.chars().all(|ch| ch.is_whitespace())
}

fn parse_ref_tokens(prompt: &str) -> Vec<ReqRefToken> {
    let mut tokens = Vec::new();
    let mut search_from = 0usize;
    let mut last_token_end: Option<usize> = None;
    while search_from < prompt.len() {
        let Some(rel) = prompt[search_from..].find(REQ_REF_PREFIX) else {
            break;
        };
        let start = search_from + rel;
        let inner_start = start + REQ_REF_PREFIX.len();
        let Some(end_rel) = prompt[inner_start..].find(']') else {
            break;
        };
        let inner_end = inner_start + end_rel;
        let inner = &prompt[inner_start..inner_end];
        if let Some(token) = parse_inner_token(inner, inner_end + 1) {
            let adjacent = match last_token_end {
                Some(prev_end) => is_adjacent_gap(&prompt[prev_end..start]),
                None => false,
            };
            let same_as_last = tokens.last().map(|last: &ReqRefToken| last.kind == token.kind && last.id == token.id).unwrap_or(false);
            if !(adjacent && same_as_last) {
                tokens.push(token);
            }
            last_token_end = Some(inner_end + 1);
        }
        search_from = inner_end + 1;
    }
    tokens
}

fn parse_inner_token(inner: &str, end: usize) -> Option<ReqRefToken> {
    let (kind_raw, id_raw) = inner.split_once(':')?;
    let id = id_raw.trim();
    if id.is_empty() {
        return None;
    }
    let kind = ReqRefKind::from_key(kind_raw)?;
    Some(ReqRefToken { kind, id: id.to_string(), end })
}

pub async fn build_referenced_prompt_messages(prompt: &str, mcp_client_pool: &McpClientPool) -> Result<Vec<Message>> {
    let mut messages = Vec::new();
    for token in parse_ref_tokens(prompt).into_iter().filter(|token| token.kind == ReqRefKind::McpPrompt) {
        let Some((server_id, name)) = token.id.split_once('/') else {
            return Err(anyhow!("referenced mcp prompt id must be server_id/name: {}", token.id));
        };
        let snapshot = mcp_client_pool.find_prompt(server_id, name)?;
        let arguments = build_prompt_arguments(&snapshot, resolve_token_tail(prompt, token.end));
        let result = mcp_client_pool.get_prompt(server_id, name, arguments).await?;
        for message in result.messages {
            messages.push(convert_prompt_message(&token.id, message)?);
        }
    }
    Ok(messages)
}

pub fn build_referenced_mcp_servers_message(prompt: &str, mcp_client_pool: &McpClientPool) -> Result<Option<String>> {
    let mut servers: Vec<ReqRefMcpServer> = Vec::new();
    for token in parse_ref_tokens(prompt).into_iter().filter(|token| token.kind == ReqRefKind::McpServer) {
        if servers.iter().any(|server| server.server_id == token.id) {
            continue;
        }
        let snapshots: Vec<_> = mcp_client_pool.list_tools().into_iter().filter(|tool| tool.server_id == token.id).collect();
        let Some(first) = snapshots.first() else {
            return Err(anyhow!("referenced mcp server is not connected or has no tools: {}", token.id));
        };
        servers.push(ReqRefMcpServer { server_id: token.id.clone(), name: first.server_name.clone(), tools: snapshots.iter().map(|tool| tool.name.clone()).collect() });
    }
    if servers.is_empty() {
        return Ok(None);
    }
    let text = serde_json::to_string(&servers)?;
    Ok(Some(format!("[Referenced MCP Servers]{text}")))
}

fn resolve_token_tail(prompt: &str, end: usize) -> &str {
    let tail = &prompt[end..];
    match tail.find('\n') {
        Some(index) => tail[..index].trim(),
        None => tail.trim(),
    }
}

fn build_prompt_arguments(snapshot: &McpPromptSnapshot, tail: &str) -> Option<JsonObject> {
    if snapshot.arguments.is_empty() || tail.is_empty() {
        return None;
    }
    let mut values = tail.split_whitespace();
    let mut object = JsonObject::new();
    for argument in &snapshot.arguments {
        let Some(value) = values.next() else {
            break;
        };
        object.insert(argument.name.clone(), Value::String(value.to_string()));
    }
    Some(object)
}

fn convert_prompt_message(token_id: &str, message: PromptMessage) -> Result<Message> {
    let text = match message.content {
        PromptMessageContent::Text { text } => text,
        PromptMessageContent::Resource { resource } => match &resource.resource {
            ResourceContents::TextResourceContents { text, .. } => text.clone(),
            ResourceContents::BlobResourceContents { uri, .. } => return Err(anyhow!("mcp prompt {} returned a binary resource that is not supported: {}", token_id, uri)),
        },
        PromptMessageContent::Image { .. } => return Err(anyhow!("mcp prompt {} returned an image message that is not supported", token_id)),
        PromptMessageContent::ResourceLink { link } => return Err(anyhow!("mcp prompt {} returned a resource link that is not supported: {}", token_id, link.uri)),
    };
    match message.role {
        PromptMessageRole::User => Ok(Message::user(text)),
        PromptMessageRole::Assistant => Ok(Message::assistant(text)),
    }
}
