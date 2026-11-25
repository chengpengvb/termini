use regex::Regex;
use std::sync::Arc;
use serde_json::Value;
use std::time::Duration;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::module::ai::dto::AiRunContext;
use rig::tool::{DynamicTool, ToolContext, ToolSet};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::shared::{ShellDaemonPush, WakeBus, WakeSource};
use crate::module::ai::tools::common::shell::local_shell_wake::match_regex_text;

const INTERNAL_CALL_ID_KEY: &str = "__internal_call_id";

pub struct LoopRunner {
    pub args: String,
    pub regex: Regex,
    pub wake_bus: WakeBus,
    pub tool: DynamicTool,
    pub interval: Duration,
    pub source: WakeSource,
    pub run_context: Arc<AiRunContext>,
    pub shell_daemon_push: ShellDaemonPush,
    pub chat_session_cache_manager: ChatSessionCacheManager,
}

impl LoopRunner {
    pub fn spawn(self) {
        tokio::spawn(async move {
            loop {
                let result = tokio::select! {
                    biased;
                    _ = self.source.stop.cancelled() => {
                        info!("[ai] 循环监控已停止 chat_id={} loop_id={}", self.source.chat_id, self.source.source_id);
                        return;
                    }
                    result = self.tick() => result,
                };
                match result {
                    Ok(output) => {
                        let Some(lines) = match_regex_text(&self.regex, &output) else {
                            continue;
                        };
                        info!("[ai] 返回命中 wake_pattern chat_id={} loop_id={}", self.source.chat_id, self.source.source_id);
                        self.wake_bus.signal(&self.source.chat_id, &self.source.source_id, format!("{} 返回命中 wake_pattern：\n{}", self.tool.name(), lines));
                    }
                    Err(error) => {
                        warn!("[ai] 调用失败, 循环监控停止 chat_id={} loop_id={} tool={} error={}", self.source.chat_id, self.source.source_id, self.tool.name(), error);
                        self.wake_bus.finish(&self.source.chat_id, &self.source.source_id, format!("调用 {} 失败，循环监控已自动停止：{}", self.tool.name(), error));
                        self.shell_daemon_push.push_daemon_list(&self.source.chat_id);
                        return;
                    }
                }
            }
        });
    }

    pub async fn invoke(&self) -> Result<String, AiToolError> {
        let call_id = uuid();
        self.chat_session_cache_manager.insert_tool_cancel(&self.run_context.req_id, self.run_context.run_id.clone(), call_id.clone()).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let args = Self::args_with_call_id(&self.args, &call_id)?;
        let mut context = ToolContext::new();
        context.insert(self.run_context.clone());
        let result = ToolSet::from_dynamic_tools(vec![self.tool.clone()]).execute(self.tool.name(), args, &mut context).await;
        if let Err(error) = self.chat_session_cache_manager.remove_tool_cancel(&self.run_context.req_id, &self.run_context.run_id, &call_id) {
            warn!("[ai] 移除 tool cancel 失败 req_id={} call_id={} error={}", self.run_context.req_id, call_id, error);
        }
        if let Some(error) = result.error() {
            return Err(AiToolError::Execution(error.to_string()));
        }
        if let Some(error) = result.refusal() {
            return Err(AiToolError::Execution(error.to_string()));
        }
        match result.output().as_text() {
            Some(text) => Ok(text.to_string()),
            None => Err(AiToolError::Execution(format!("tool {} output is not text", self.tool.name()))),
        }
    }

    async fn tick(&self) -> Result<String, AiToolError> {
        tokio::time::sleep(self.interval).await;
        self.invoke().await
    }

    fn args_with_call_id(args: &str, call_id: &str) -> Result<String, AiToolError> {
        let value: Value = serde_json::from_str(args).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Value::Object(mut map) = value else {
            return Err(AiToolError::Execution("args must be a JSON object".to_string()));
        };
        map.insert(INTERNAL_CALL_ID_KEY.to_string(), Value::String(call_id.to_string()));
        serde_json::to_string(&Value::Object(map)).map_err(|error| AiToolError::Execution(error.to_string()))
    }
}
