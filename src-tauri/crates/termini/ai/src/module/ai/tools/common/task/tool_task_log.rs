use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::task::session::task_registry::TaskRegistry;

const LOG_DEFAULT_TAIL: usize = 30;
const LOG_MAX_TAIL: usize = 200;
const LOG_MAX_CHARS: usize = 6000;
const RESULT_MAX_LINES: usize = 1000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolTaskLogArgs {
    pub task_id: String,

    #[serde(default)]
    pub tail_lines: Option<usize>,

    #[serde(default)]
    pub include_result: Option<bool>,

    #[serde(default)]
    pub offset: Option<i64>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 765)]
#[derive(Clone, Service)]
pub struct ToolTaskLog {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolTaskLog {
    fn name(&self) -> &'static str {
        "ToolTaskLog"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolTaskLog {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolTaskLog
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolTaskLog {
    const NAME: &'static str = AiToolId::ToolTaskLog.as_str();

    type Error = AiToolError;
    type Args = ToolTaskLogArgs;
    type Output = String;

    fn description(&self) -> String {
        "读取某个后台任务的进度：子代理给出的是调查轨迹，图片识别给出的是已转写出的文字（可据此判断是否在重复同一段话或跑偏，是则 task_kill）。task_id 来自 task_list 或 task/image_read 转后台时返回的那个 id。默认只给最近 30 行，要更多传 tail_lines（上限 200）；任务已结束时传 include_result=true 才取回最终结果，用 offset（从 1 起的行号，负数从末尾倒数）与 limit（最多行数，默认最多 1000 行）翻页，看 showing lines X-Y of Z 继续读。禁止一次把全部进度拉进上下文。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "要读取的后台任务 id"
                },
                "tail_lines": {
                    "type": "integer",
                    "description": "最多返回多少行进度，默认 30，上限 200，取最新的那些"
                },
                "include_result": {
                    "type": "boolean",
                    "description": "任务已结束时是否附带最终结果，默认 false"
                },
                "offset": {
                    "type": "integer",
                    "description": "仅 include_result=true 时有效：最终结果起始行号，从 1 开始；负数表示从末尾倒数。不传则从第 1 行开始"
                },
                "limit": {
                    "type": "integer",
                    "description": "仅 include_result=true 时有效：最多返回最终结果行数，默认最多 1000 行"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(session) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session.context.ai_chat_id.clone();
        drop(session);

        let Some(entry) = self.task_registry.get(&chat_id, &args.task_id) else {
            return Err(AiToolError::Execution(format!("task_id={} is not among this chat's background tasks", args.task_id)));
        };

        let limit = args.tail_lines.unwrap_or(LOG_DEFAULT_TAIL).clamp(1, LOG_MAX_TAIL);
        let progress = entry.progress.lock().clone();
        let rows: Vec<&str> = progress.lines().collect();
        let skip = rows.len().saturating_sub(limit);
        let tail = rows[skip..].join("\n");
        let body = if tail.trim().is_empty() { "(no progress yet)".to_string() } else { fit_tail(&tail, LOG_MAX_CHARS) };

        let mut out = format!("{}\n[最近 {} 行, 共 {} 行]\n{}", entry.summary_text(), rows.len() - skip, rows.len(), body);
        if args.include_result.unwrap_or(false) {
            match entry.result.lock().clone() {
                Some(text) => out.push_str(&format!("\n\n{}", Self::page_result(&text, args.offset, args.limit)?)),
                None => out.push_str("\n\n[最终结果] 还没有(任务未结束)"),
            }
        } else if entry.state().is_finished() && entry.result.lock().is_some() {
            out.push_str("\n\n(任务已结束, 需要完整结果请再调一次并传 include_result=true)");
        }
        context.insert_result(vec![WrapItem::wrap_item(None, out.clone())]);
        Ok(out)
    }
}

impl ToolTaskLog {
    fn resolve_start(offset: Option<i64>, total: usize) -> usize {
        match offset {
            Some(value) if value < 0 => total.saturating_sub((-value) as usize),
            Some(value) if value <= 1 => 0,
            Some(value) => (value as usize).saturating_sub(1).min(total),
            None => 0,
        }
    }

    fn page_result(text: &str, offset: Option<i64>, limit: Option<usize>) -> Result<String, AiToolError> {
        let lines: Vec<&str> = text.lines().collect();
        let total = lines.len();
        if total == 0 {
            return Ok("[最终结果] showing lines 0-0 of 0".to_string());
        }
        let start_index = Self::resolve_start(offset, total);
        if start_index >= total {
            return Err(AiToolError::Execution(format!("offset past end of result, total lines={}", total)));
        }
        let take = limit.unwrap_or(RESULT_MAX_LINES).min(RESULT_MAX_LINES);
        if take == 0 {
            return Err(AiToolError::Execution("limit must be greater than 0".to_string()));
        }
        let end_index = (start_index + take).min(total);
        let start_line = start_index + 1;
        let mut out = format!("[最终结果] showing lines {}-{} of {}\n{}", start_line, end_index, total, lines[start_index..end_index].join("\n"));
        if end_index < total {
            out.push_str(&format!("\n… truncated, showing lines {}-{} of {}", start_line, end_index, total));
        }
        Ok(out)
    }
}

fn fit_tail(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    text.chars().skip(count - max_chars).collect()
}
