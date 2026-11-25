use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use std::time::Duration;
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
use crate::module::ai::tools::common::task::session::task_registry::{TaskEntry, TaskRegistry};

const WAIT_DEFAULT_SECS: u64 = 60;
const WAIT_MAX_SECS: u64 = 600;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolTaskWaitArgs {
    #[serde(default)]
    pub task_id: Option<String>,

    #[serde(default)]
    pub timeout_seconds: Option<u64>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolTaskWait {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolTaskWait {
    fn name(&self) -> &'static str {
        "ToolTaskWait"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolTaskWait {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolTaskWait
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolTaskWait {
    const NAME: &'static str = AiToolId::ToolTaskWait.as_str();

    type Error = AiToolError;
    type Args = ToolTaskWaitArgs;
    type Output = String;

    fn description(&self) -> String {
        "挂起等待后台任务（子代理调查或转后台的图片识别）结束：不传 task_id 就等任意一个先结束，传了就等那一个。有任务结束立即返回，超时(默认 60 秒，上限 600)则返回当时的状态汇总(已运行/静默/进度行数)，据此判断它是否还在推进。等待期间不耗轮次。还有后台任务在跑、而你手上没有别的事可做时用它顶住本回合，不要因为闲着就收尾——那会让用户拿不到后台结果。不要用 shell sleep 代替本工具。只管子代理和图片/PDF 任务：shell_exec 转异步的命令用 shell_process_log 传 wait_seconds 等；loop_start 和传了 wake_pattern 的命令不用等，它们结束或命中规则会主动唤醒你。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "只等这一个；不传则等任意一个先结束"
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "最长等待秒数，默认 60，上限 600。超时会返回状态，可再次调用继续等"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(session) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session.context.ai_chat_id.clone();
        drop(session);

        let targets: Vec<TaskEntry> = match args.task_id.as_deref() {
            Some(task_id) => {
                let Some(entry) = self.task_registry.get(&chat_id, task_id) else {
                    return Err(AiToolError::Execution(format!("task_id={} is not among this chat's background tasks", task_id)));
                };
                vec![entry]
            }
            None => self.task_registry.list_detached_running(&chat_id),
        };
        let body = if targets.is_empty() {
            "[task_wait] no background tasks are still running, nothing to wait for".to_string()
        } else if let Some(done) = targets.iter().find(|entry| entry.state().is_finished()) {
            format!("[task_wait] 已结束\n{}\n用 task_log 加 include_result=true 取回结果", done.summary_text())
        } else {
            let timeout = Duration::from_secs(args.timeout_seconds.unwrap_or(WAIT_DEFAULT_SECS).clamp(1, WAIT_MAX_SECS));
            let session_cancel = run_context.cancel_token.clone();
            // 先登记 notified() 再检查状态, 否则会漏掉登记与检查之间结束的任务
            let waiters: Vec<_> = targets.iter().map(|entry| entry.finished.clone()).collect();
            let any_finished = async {
                let mut futures: Vec<_> = waiters.iter().map(|notify| Box::pin(notify.notified())).collect();
                for future in futures.iter_mut() {
                    future.as_mut().enable();
                }
                if targets.iter().any(|entry| entry.state().is_finished()) {
                    return;
                }
                futures::future::select_all(futures).await;
            };

            tokio::select! {
                biased;
                _ = session_cancel.cancelled() => "[task_wait] cancelled".to_string(),
                _ = any_finished => {
                    let rows: Vec<String> = targets.iter().map(|entry| entry.summary_text()).collect();
                    format!("[task_wait] 有任务结束了\n{}\n用 task_log 加 include_result=true 取回结果", rows.join("\n"))
                }
                _ = tokio::time::sleep(timeout) => {
                    let rows: Vec<String> = targets.iter().map(|entry| entry.summary_text()).collect();
                    format!("[task_wait] 等待 {}s 超时, 都还在跑\n{}\n进度行数还在涨就是在推进, 没别的事可做就再调一次继续等; 静默几分钟且进度不再增长才用 task_kill", timeout.as_secs(), rows.join("\n"))
                }
            }
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
