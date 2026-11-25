use regex::Regex;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use serde_json::{json, Value};
use crate::util::id_factory::uuid;
use summer::plugin::LazyComponent;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::wake::loop_runner::LoopRunner;
use crate::module::ai::shared::{ShellDaemonPush, WakeBus, WakeSource, WakeSourceKind};
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem, TOOL_OUTPUT_MAX_TOKENS};

const INTERVAL_MIN_SECS: u64 = 5;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolLoopStartArgs {
    pub tool: String,

    pub title: String,

    #[serde(default)]
    pub args: Value,

    pub wake_pattern: String,

    pub interval_seconds: u64,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 763)]
#[derive(Clone, Service)]
pub struct ToolLoopStart {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    shell_daemon_push: ShellDaemonPush,

    mcp_client_pool: LazyComponent<McpClientPool>,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolLoopStart {
    fn name(&self) -> &'static str {
        "ToolLoopStart"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolLoopStart {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolLoopStart
    }

    fn persist(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolLoopStart {
    fn validate(args: &ToolLoopStartArgs) -> Result<Regex, AiToolError> {
        if args.title.trim().is_empty() {
            return Err(AiToolError::Execution("title is empty".to_string()));
        }
        if args.tool.trim().is_empty() {
            return Err(AiToolError::Execution("tool is empty".to_string()));
        }
        if !matches!(args.args, Value::Object(_) | Value::Null) {
            return Err(AiToolError::Execution("args must be a JSON object".to_string()));
        }
        if args.interval_seconds < INTERVAL_MIN_SECS {
            return Err(AiToolError::Execution(format!("interval_seconds must be at least {}", INTERVAL_MIN_SECS)));
        }
        Regex::new(&args.wake_pattern).map_err(|error| AiToolError::Execution(format!("wake_pattern is not a valid regex: {}", error)))
    }

    fn resolve_tool(&self, req_id: &str, name: &str) -> Result<DynamicTool, AiToolError> {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", req_id)));
        };
        let builtin = entry.tools.iter().find(|tool| tool.tool_id().as_str() == name && tool.domains().contains(&ToolDomain::Main)).map(|tool| tool.as_rig_tool(None));
        drop(entry);
        if let Some(tool) = builtin {
            return Ok(tool);
        }
        let mcp_client_pool = self.mcp_client_pool.get().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let mcp = mcp_client_pool.collect_rig_tools().into_iter().find(|tool| tool.name() == name).ok_or_else(|| AiToolError::Execution(format!("unknown tool: {}", name)))?;
        Ok(ToolBaseOutWrap::wrap(mcp, TOOL_OUTPUT_MAX_TOKENS, None))
    }
}

impl Tool for ToolLoopStart {
    const NAME: &'static str = AiToolId::ToolLoopStart.as_str();

    type Args = ToolLoopStartArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "把一个工具调用变成后台循环监控（后台唤醒源）：框架每隔 interval_seconds 秒用同样的 args 调用一次 tool，返回中任一整行命中 wake_pattern 正则时，以一条 [后台唤醒] 新消息叫醒你并附上命中的行；某次调用失败也会叫醒你并自动停止。用于不方便写 shell 脚本自己循环的场景：MCP 浏览器工具盯页面、web_fetch 盯网页、远程只读命令盯状态。目标工具在后台无对话上下文下调用，只能是 MCP 工具或不依赖对话的工具；本机 shell 监控不要用它，改用 shell_exec 传 wake_pattern 让脚本自己循环。调用时先立即执行一次并把结果返回给你（用它确认 args 和 wake_pattern 写对了），然后转入后台。启动后没有别的事就收尾并说明在等什么；目标达成后用 loop_stop 停掉，用户也可在界面停止，停止后不再唤醒。".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "这个监控在盯什么的短标题，语言遵循 [输出语言]，界面上用它标识这一条"
                },
                "tool": {
                    "type": "string",
                    "description": "要循环调用的工具名，MCP 工具直接写它的名字"
                },
                "args": {
                    "type": "object",
                    "description": "每次调用传给该工具的参数，与直接调用它时相同"
                },
                "interval_seconds": {
                    "type": "integer",
                    "description": "两次调用的间隔秒数，最小 5；按目标变化的快慢给，不要为了快而设得很小"
                },
                "wake_pattern": {
                    "type": "string",
                    "description": "正则；返回文本中任一整行命中就叫醒你。只匹配真正表示变化或到达条件的内容，不要给每次都会出现的词"
                }
            },
            "required": ["title", "tool", "interval_seconds", "wake_pattern"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let regex = Self::validate(&args)?;
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let Some(session) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_context = session.context.clone();
        drop(session);
        let tool = self.resolve_tool(&run_context.req_id, &args.tool)?;
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &args).await?;

        let target_args = match &args.args {
            Value::Null => "{}".to_string(),
            other => serde_json::to_string(other).map_err(|error| AiToolError::Execution(error.to_string()))?,
        };
        let runner = LoopRunner {
            tool,
            args: target_args,
            wake_bus: self.wake_bus.clone(),
            interval: Duration::from_secs(args.interval_seconds),
            source: WakeSource::new(WakeSourceKind::Loop, uuid(), args.title.trim().to_string(), Some(regex.clone()), chat_context),
            regex,
            run_context: run_context.clone(),
            shell_daemon_push: self.shell_daemon_push.clone(),
            chat_session_cache_manager: self.chat_session_cache_manager.clone(),
        };
        let first = tokio::select! {
            biased;
            _ = tool_cancel.cancelled() => {
                let body = "exit_code=cancelled".to_string();
                context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
                return Ok(body);
            }
            _ = run_context.cancel_token.cancelled() => {
                let body = "exit_code=cancelled".to_string();
                context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
                return Ok(body);
            }
            result = runner.invoke() => result.map_err(|error| AiToolError::Execution(format!("first call of {} failed, loop not started: {}", args.tool, error)))?,
        };

        let loop_id = runner.source.source_id.clone();
        let chat_id = runner.source.chat_id.clone();
        self.wake_bus.register(runner.source.clone());
        runner.spawn();
        self.shell_daemon_push.push_daemon_list(&chat_id);
        let body = format!(
            "loop_id={} 循环监控「{}」已转入后台：每 {} 秒调用一次 {}，返回中任一整行命中 /{}/ 时会以 [后台唤醒] 新消息叫醒你并附上命中的行，调用失败也会叫醒你并自动停止。目标达成后用 {} 停掉它。首次调用返回：\n{}",
            loop_id,
            args.title.trim(),
            args.interval_seconds,
            args.tool,
            args.wake_pattern,
            AiToolId::ToolLoopStop.as_str(),
            first
        );
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
