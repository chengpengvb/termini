use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use std::time::Duration;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use tokio::time::{sleep, Instant};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::shell::local_shell_wake::WAKE_POLL_INTERVAL;
use crate::module::ai::tools::common::shell::local_shell_registry::{LocalShellRegistry, ShellProcessEntry};

const LOG_DEFAULT_TAIL: usize = 200;
const LOG_MAX_CHARS: usize = 8000;
const WAIT_MAX_SECS: u64 = 600;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolShellProcessLogArgs {
    pub process_id: String,

    #[serde(default)]
    pub since_seq: Option<u64>,

    #[serde(default)]
    pub tail_lines: Option<usize>,

    #[serde(default)]
    pub wait_seconds: Option<u64>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolShellProcessLog {
    #[inject(func = bool::default())]
    gated: bool,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolShellProcessLog {
    fn name(&self) -> &'static str {
        "ToolShellProcessLog"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolShellProcessLog {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolShellProcessLog
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }

    fn as_gated_tool(&self) -> DynamicTool {
        ToolBaseOutWrap::wrap(Self { gated: true, ..self.clone() }, self.max_result_tokens(), None)
    }
}

impl Tool for ToolShellProcessLog {
    const NAME: &'static str = AiToolId::ToolShellProcessLog.as_str();

    type Args = ToolShellProcessLogArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        let head = "读取某个本地命令进程的回显，process_id 来自 shell_process_list 或 shell_exec 转异步/挂载时返回的那个 id。返回开头一行是该进程的状态汇总，之后是回显正文。进程转到后台之后，它新增的输出只能用本工具读到。";
        let tail = "静默秒数看状态汇总：长时间不前进，说明进程停在原地。";
        if self.gated {
            return format!("{}{}", head, tail);
        }
        format!("{}传 wait_seconds 时先挂起等它结束（最多该秒数，上限 600，等待期间不耗轮次），进程退出或超时后再返回：这是拿转异步命令结果的方式，等价于 sleep 后读结果，不要用 shell sleep 或反复轮询代替；状态汇总里状态仍是 running 就是超时未结束，可以再等一次。{}", head, tail)
    }

    fn parameters(&self) -> serde_json::Value {
        let mut properties = json!({
            "process_id": {
                "type": "string",
                "description": "要读取的进程 id"
            },
            "since_seq": {
                "type": "integer",
                "description": "只要该偏移之后的输出；不传则取最近的行"
            },
            "tail_lines": {
                "type": "integer",
                "description": "最多返回多少行，默认 200，取最新的那些"
            }
        });
        if !self.gated {
            properties["wait_seconds"] = json!({
                "type": "integer",
                "description": "先挂起等进程结束再读，最多等这么多秒（上限 600）；不传立即读当前回显"
            });
        }
        json!({
            "type": "object",
            "properties": properties,
            "required": ["process_id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(session_entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session_entry.context.ai_chat_id.clone();
        drop(session_entry);
        let Some(entry) = self.local_shell_registry.get(&chat_id, &args.process_id) else {
            return Err(AiToolError::Execution(format!("process_id={} is not among this chat's local command processes", args.process_id)));
        };
        if let Some(seconds) = args.wait_seconds {
            if self.gated {
                return Err(AiToolError::Execution("wait_seconds is not available to the supervisor: decide from the current output instead of waiting for the process".to_string()));
            }
            let Some(call_id) = args.internal_call_id.as_deref() else {
                return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
            };
            self.wait_for_exit(&run_context, call_id, &entry, seconds).await;
        }
        let limit = args.tail_lines.unwrap_or(LOG_DEFAULT_TAIL).max(1);
        let output = entry.chunk_buffer.lock().clone();
        let empty = format!("{}\n(no matching output)", entry.summary_text());
        let start = match args.since_seq {
            Some(seq) => {
                let start = seq as usize;
                if start > output.len() {
                    None
                } else if !output.is_char_boundary(start) {
                    return Err(AiToolError::Execution(format!("since_seq={} is not a valid offset", seq)));
                } else {
                    Some(start)
                }
            }
            None => Some(0),
        };
        let body = match start {
            None => empty.clone(),
            Some(start) => {
                let slice = &output[start..];
                if slice.is_empty() {
                    empty
                } else {
                    let rows: Vec<&str> = slice.lines().collect();
                    let skip = rows.len().saturating_sub(limit);
                    let text = rows[skip..].join("\n");
                    if text.is_empty() {
                        empty
                    } else {
                        format!("{}\n{}", entry.summary_text(), Self::fit_budget(&text, LOG_MAX_CHARS))
                    }
                }
            }
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolShellProcessLog {
    async fn wait_for_exit(&self, run: &AiRunContext, call_id: &str, entry: &ShellProcessEntry, seconds: u64) {
        let mut process = entry.process.clone();
        let mut cursor = entry.chunk_buffer.lock().len();
        let deadline = Instant::now() + Duration::from_secs(seconds.clamp(1, WAIT_MAX_SECS));
        loop {
            self.push_added(run, call_id, entry, &mut cursor);
            if entry.process.exit_status().is_some() || run.cancel_token.is_cancelled() {
                return;
            }
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            tokio::select! {
                biased;
                _ = run.cancel_token.cancelled() => return,
                _ = process.wait_exit() => {
                    self.push_added(run, call_id, entry, &mut cursor);
                    return;
                }
                _ = sleep(WAKE_POLL_INTERVAL.min(deadline - now)) => {}
            }
        }
    }

    fn push_added(&self, run: &AiRunContext, call_id: &str, entry: &ShellProcessEntry, cursor: &mut usize) {
        let buffer = entry.chunk_buffer.lock();
        if buffer.len() <= *cursor {
            return;
        }
        let delta = buffer[*cursor..].to_string();
        *cursor = buffer.len();
        drop(buffer);
        if let Err(error) = self.tool_call_chunk_push.push(run, call_id, AiToolId::ToolShellProcessLog.as_str(), delta) {
            warn!("[ai] 推送等待回显失败 call_id={} error={:#}", call_id, error);
        }
    }

    fn fit_budget(text: &str, max_chars: usize) -> String {
        let count = text.chars().count();
        if count <= max_chars {
            return text.to_string();
        }
        text.chars().skip(count - max_chars).collect()
    }
}
