use std::sync::Arc;
use serde_json::Value;
use std::path::{Path, PathBuf};
use crate::util::id_factory::uuid;
use crate::util::ansi_util::strip_ansi;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ChatFileStore;
use crate::module::ai::shared::ToolCallDonePush;
use super::tool_base_out_wrap_inner::ToolBaseOutWrapInner;
use crate::module::ai::shared::chat_file_store::CHAT_OUTPUTS_DIR;
use crate::util::token_util::{clamp_text_tokens, estimate_text_tokens};
use rig::tool::{DynamicTool, ToolContext, ToolExecutionError, ToolOutput, ToolResult};

pub const TOOL_OUTPUT_MAX_TOKENS: usize = 10000;
pub const INTERNAL_CALL_ID_KEY: &str = "__internal_call_id";
const CLAMP_NOTE_PERSIST: &str = "用 output_read 传该 id 查阅全文，禁止用相同参数重跑";
const CLAMP_NOTE_REQUERY: &str = "缩小本工具参数取被省略段，无 truncated id";

#[derive(Clone)]
pub struct WrapItem {
    pub id: String,
    pub body: String,
    pub title: Option<String>,
}

impl WrapItem {
    pub fn wrap_item(title: Option<String>, body: String) -> Self {
        WrapItem { id: uuid(), title, body }
    }
}

pub struct ToolBaseOutWrap;

impl ToolBaseOutWrap {
    pub fn wrap(inner: impl ToolBaseOutWrapInner, max_tokens: usize, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        let persist = inner.persist();
        let name = inner.tool_name();
        let parameters = inner.tool_parameters();
        let description = inner.tool_description();
        DynamicTool::new(name.clone(), description, parameters, move |context, args| {
            let name = name.clone();
            let inner = inner.clone();
            let done_push = done_push.clone();
            Box::pin(async move {
                let result = inner.into_set().execute(&name, args.to_string(), context).await;
                Self::take_output(result)?;
                let items = match context.result::<Vec<WrapItem>>() {
                    Some(items) => items.clone(),
                    None => return Err(ToolExecutionError::other("wrap requires Vec<WrapItem>".to_string())),
                };
                let spill_dir = if persist { Self::spill_dir(context) } else { None };
                let gated = Self::gate_items(items, max_tokens, spill_dir, persist)?;
                if let Some(done_push) = &done_push {
                    Self::push_done(done_push, &args, context, &gated);
                }
                Ok(ToolOutput::text(gated))
            })
        })
    }

    fn gate_items(items: Vec<WrapItem>, max_tokens: usize, spill_dir: Option<PathBuf>, persist: bool) -> Result<String, ToolExecutionError> {
        if items.is_empty() {
            return Err(ToolExecutionError::other("wrap items is empty".to_string()));
        }
        let items: Vec<WrapItem> =
            items.into_iter().map(|item| WrapItem { id: item.id, body: strip_ansi(&item.body), title: item.title.map(|title| strip_ansi(&title)).filter(|title| !title.is_empty()) }).collect();
        if items.iter().any(|item| item.id.is_empty()) {
            return Err(ToolExecutionError::other("wrap item id is empty".to_string()));
        }
        let title_cost: usize = items.iter().map(|item| item.title.as_deref().map(estimate_text_tokens).unwrap_or(0)).sum();
        if title_cost > max_tokens {
            return Err(ToolExecutionError::other(format!("wrap item titles exceed {} tokens", max_tokens)));
        }
        let mut left = items.len();
        let mut parts: Vec<String> = Vec::new();
        let mut remaining = max_tokens - title_cost;
        for item in items {
            let share = if left > 1 { remaining / left } else { remaining };
            let cost = estimate_text_tokens(&item.body);
            let part = if cost <= share {
                remaining = remaining.saturating_sub(cost);
                Self::render_item(item.title.as_deref(), &item.body, None)
            } else {
                remaining = remaining.saturating_sub(share);
                let body = clamp_text_tokens(item.body.clone(), share);
                let mut part = if persist {
                    let dir = spill_dir.as_ref().ok_or_else(|| ToolExecutionError::other("wrap spill dir is missing".to_string()))?;
                    Self::spill_item(dir, &item.id, item.title.as_deref(), &item.body)?;
                    Self::render_item(item.title.as_deref(), &body, Some(&item.id))
                } else {
                    Self::render_item(item.title.as_deref(), &body, None)
                };
                part.push('\n');
                part.push_str(if persist { CLAMP_NOTE_PERSIST } else { CLAMP_NOTE_REQUERY });
                part
            };
            left = left.saturating_sub(1);
            parts.push(part);
        }
        Ok(parts.join("\n"))
    }

    fn render_item(title: Option<&str>, body: &str, truncated_id: Option<&str>) -> String {
        let mut out = String::new();
        if let Some(title) = title {
            out.push_str(title);
            out.push('\n');
        }
        out.push_str(body);
        if let Some(id) = truncated_id {
            out.push_str(&format!("\n[truncated id={}]", id));
        }
        out
    }

    fn spill_dir(context: &ToolContext) -> Option<PathBuf> {
        let run_context = context.get::<Arc<AiRunContext>>()?;
        let chat_id = run_context.chat_id.as_deref()?;
        Some(ChatFileStore::chat_dir(chat_id).ok()?.join(CHAT_OUTPUTS_DIR))
    }

    fn spill_item(dir: &Path, id: &str, title: Option<&str>, body: &str) -> Result<(), ToolExecutionError> {
        std::fs::create_dir_all(dir).map_err(|error| ToolExecutionError::other(error.to_string()))?;
        let path = dir.join(format!("{}.txt", id));
        let text = match title {
            Some(title) => format!("{}\n{}", title, body),
            None => body.to_string(),
        };
        std::fs::write(&path, &text).map_err(|error| ToolExecutionError::other(error.to_string()))?;
        Ok(())
    }

    fn push_done(done_push: &ToolCallDonePush, args: &Value, context: &ToolContext, result: &str) {
        let Ok(run_context) = context.require::<Arc<AiRunContext>>() else {
            return;
        };
        let Some(call_id) = args.get(INTERNAL_CALL_ID_KEY).and_then(|id| id.as_str()) else {
            return;
        };
        done_push.push_tool_call_done_event(run_context, call_id, result.to_string());
    }

    fn take_output(result: ToolResult) -> Result<ToolOutput, ToolExecutionError> {
        if let Some(error) = result.error() {
            return Err(error.clone());
        }
        if let Some(error) = result.refusal() {
            return Err(error.clone());
        }
        Ok(result.output().clone())
    }
}
