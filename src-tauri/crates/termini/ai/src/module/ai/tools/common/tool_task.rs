use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::AiTaskSession;
use crate::module::ai::shared::SubagentToolPush;
use crate::module::ai::shared::ToolCallDonePush;
use crate::service::service_agent::AgentService;
use rig::tool::{DynamicTool, Tool, ToolContext};
use rig::completion::{AssistantContent, Message};
use crate::module::ai::agent::agent_main::AgentMain;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::common::task::rule::scope;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::agent::prompts::output_language_addons;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::task::session::task_result;
use crate::module::ai::tools::common::task::prompt::subagent_prompt;
use crate::service::service_impl_system_util::SystemUtilServiceImpl;
use crate::module::ai::tools::common::task::prompt::parent_background;
use crate::module::ai::tools::common::task::session::subagent_addon::SubagentRunAddon;
use crate::module::ai::tools::common::task::session::task_registry::{TaskEntry, TaskKind, TaskRegistry, TaskState};
use crate::module::ai::tools::common::task::session::task_coordinator::{TaskCoordinator, TaskOutcome, BLOCK_MAX_SECS};

const REPORT_PROMPT: &str = "材料已经够了。不要再调用任何工具，直接输出最终报告。";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolTaskArgs {
    pub prompt: String,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub scope: Option<Vec<String>>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 763)]
#[derive(Clone, Service)]
pub struct ToolTask {
    #[inject(component)]
    agent_main: AgentMain,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    ai_task_session: AiTaskSession,

    #[inject(component)]
    task_coordinator: TaskCoordinator,

    #[inject(component)]
    subagent_tool_push: SubagentToolPush,

    #[inject(component)]
    system_util_service: SystemUtilServiceImpl,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolTask {
    fn name(&self) -> &'static str {
        "ToolTask"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolTask {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolTask
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn on_stop(&self, req_id: &str) {
        self.task_coordinator.release(req_id);
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolTask {
    const NAME: &'static str = AiToolId::ToolTask.as_str();

    type Args = ToolTaskArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "派出一个子任务代理，把一段可以独立完成的工作隔离到自己的上下文里做完再交报告。子代理和你是同一套能力：同样的工具（可以查代码、读文档、改文件、跑命令、操作远程设备）、同样的审批档位，需要审批的操作会直接弹给用户点允许或跳过，你不需要替它审批；它看不到本会话历史，也不能再派 task。适合交出去的：展开读很多文件、会产生大量中间上下文的调查；边界清楚、能自己验证收尾的独立子任务（改一个模块并跑通测试、整理一份文档、批量处理一类文件）。可同时派出多个分头做：同一批里有一个先交报告后，其余的很快会转后台并把 task_id 交给你，先消化到手的报告，再用 task_wait 等剩下的。**分头派时每个子代理都要传 title（各自不同的短标题）和 scope（限定它负责的目录）**：没有 title 界面上几条会长得一模一样分不清；没有 scope 它在自己模块里搜不到东西就会去翻别的模块。scope 里的目录必须真实存在（先用 code_map 或列目录确认过），写错一个路径子代理会反复撞墙。prompt 必须自包含：目标、已知线索（路径/关键词/落点）、期望产出、验收或停止条件；一次只派一条主线，不要把五六个互不相关的事塞给同一个子代理。查代码时不要写精读或通读，子代理默认假设驱动、够回答就收；也不要替它规定工具顺序（例如「先 code_map 再 code_search」），选哪个工具由它自己判断。用户消息里的 @[localpath] 长文/附件必须用本工具，并在 prompt 里写明需要通读该文件。普通小文件仍直接 file_read。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "这个子任务的短标题，语言遵循 [输出语言]。分头派多个子代理时必须各自不同，否则界面上分不清谁在做什么"
                },
                "prompt": {
                    "type": "string",
                    "description": "自包含的子任务：目标、已知线索（路径/关键词/落点）、期望产出、验收或停止条件。若是 @[localpath] 长文，写明路径、用户问题，并要求从文件头分页读（limit 约 200）后交精炼结论"
                },
                "scope": {
                    "type": "array",
                    "description": "子任务范围的绝对路径列表，写目录不要写文件（文件会被折算成所在目录），目录必须真实存在。只要任务限定在某个模块/子目录（例如只改 gather），就必须传：子代理的工具会被程序限制在这些目录内，越界的调用直接被挡回。prompt 里引用到的符号所在文件也要在范围内，否则子代理搜不到只能报未找到。只有通读单个文件或确实需要跨全仓时才可省略",
                    "items": {"type": "string"}
                }
            },
            "required": ["prompt"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        if args.prompt.trim().is_empty() {
            return Err(AiToolError::Execution("prompt is empty".to_string()));
        }
        if self.chat_session_cache_manager.parent_req_id(&run_context.req_id).is_some() {
            return Err(AiToolError::Execution("a subagent cannot dispatch task; finish this part yourself and describe the rest in your report".to_string()));
        }
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let (chat_id, background) = {
            let Some(session) = self.chat_session_cache_manager.get(&run_context.req_id) else {
                return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
            };
            let messages = session.memory.lock().clone();
            let summary = session.summary.lock().clone();
            (session.context.ai_chat_id.clone(), parent_background::parent_background(&session.context.message, &summary, &messages))
        };
        let card_item_id = self.chat_session_cache_manager.get_tool_item_id(&run_context.req_id, &call_id);
        let task_entry = TaskEntry::new(TaskKind::Subagent, run_context.req_id.clone(), chat_id, uuid(), call_id.clone(), args.prompt.clone(), session_cancel.child_token());
        self.task_registry.insert(task_entry.clone());
        let nested = self.ai_task_session.open_nested(&run_context.req_id, task_entry.cancel.clone()).map_err(|error| AiToolError::Execution(error.to_string()))?;
        if let Err(error) = self.subagent_tool_push.push_open(&run_context, &call_id, &nested.req_id, args.title.clone()) {
            warn!("[ai] 推送子代理会话事件失败 call_id={} error={:#}", call_id, error);
        }

        let requested: Vec<String> = args.scope.clone().unwrap_or_default().into_iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect();
        let scope = scope::directories_of(&requested, |path| std::path::Path::new(path).is_file());
        if scope.len() != requested.len() {
            info!("[ai] scope 含文件条目, 已折算为目录 requested={} dirs={:?}", requested.len(), scope);
        }
        let environment =
            self.system_util_service.system_info().await.ok().map(|info| subagent_prompt::Environment { os: info.platform_info.os, terminal: info.terminal.unwrap_or_else(|| "未知".to_string()) });
        let subagent_prompt = subagent_prompt::build(&scope, &requested, environment.as_ref(), &background, &args.prompt);
        let addon = Arc::new(SubagentRunAddon::new(scope, task_entry.clone()));
        let addon_messages = output_language_addons(&run_context.addon_messages);

        let job_entry = task_entry.clone();
        let job_session = self.ai_task_session.clone();
        let job_agent = self.agent_main.clone();
        let job_agent_service = self.agent_service.clone();
        let job_parent = Arc::clone(&run_context);
        let job_addon_messages = addon_messages.clone();
        tokio::spawn(async move {
            let mut output = job_session.run_with_addon::<_, String>(&nested, &job_agent, subagent_prompt, job_addon_messages.clone(), Some(addon.clone())).await;
            if output.is_ok() && !job_entry.cancel.is_cancelled() && !last_assistant_has_text(&nested.memory.lock()) {
                info!("[ai] 子代理最后一轮没有正文, 补问一次要报告 task_id={}", job_entry.task_id);
                output = job_session.run_with_addon::<_, String>(&nested, &job_agent, REPORT_PROMPT.to_string(), job_addon_messages, Some(addon)).await;
            }
            if job_entry.cancel.is_cancelled() {
                job_entry.finish(TaskState::Cancelled, Some(task_result::CANCELLED_RESULT.to_string()));
            } else {
                match output {
                    Ok(text) => job_entry.finish(TaskState::Done, Some(text)),
                    Err(error) => job_entry.finish(TaskState::Failed, Some(task_result::failed_text(&job_entry, &error))),
                }
            }
            job_session.close_nested(nested);
            task_result::push_result(&job_agent_service, &job_parent, card_item_id, &job_entry);
        });

        let outcome = self.task_coordinator.join(&run_context.req_id, session_cancel, task_entry.clone(), tool_cancel, BLOCK_MAX_SECS).await;
        let body = match outcome {
            TaskOutcome::Detached => task_result::detached_text(&task_entry),
            TaskOutcome::Cancelled => {
                task_entry.cancel.cancel();
                task_entry.finish(TaskState::Cancelled, Some(task_result::CANCELLED_RESULT.to_string()));
                task_result::CANCELLED_RESULT.to_string()
            }
            TaskOutcome::Finished => {
                let result = task_entry.result.lock().clone().unwrap_or_default();
                if task_entry.state() == TaskState::Done {
                    self.task_registry.remove(&task_entry.chat_id, &task_entry.task_id);
                }
                result
            }
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

fn last_assistant_has_text(messages: &[Message]) -> bool {
    let Some(last) = messages.iter().rev().find(|message| matches!(message, Message::Assistant { .. })) else {
        return false;
    };
    let Message::Assistant { content, .. } = last else {
        return false;
    };
    content.iter().any(|part| matches!(part, AssistantContent::Text(text) if !text.text.trim().is_empty()))
}
