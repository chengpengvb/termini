use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::service::service_agent::AgentService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::tools::common::task::session::task_registry::TaskRegistry;
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentRunBase, RespAgentToolCallDone, RespAgentToolStatus};

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct ToolCallDonePush {
    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolCallDonePush {
    fn name(&self) -> &'static str {
        "ToolCallDonePush"
    }
}

impl ToolCallDonePush {
    pub fn push_tool_call_done_event(&self, context: &AiRunContext, call_id: &str, result: String) {
        let Ok(should_push) = self.chat_session_cache_manager.mark_tool_done(&context.req_id, call_id) else {
            warn!("[ai] 标记工具完成失败 req_id={} call_id={}", context.req_id, call_id);
            return;
        };
        if !should_push {
            return;
        }
        let Some(chat_session_cache_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            warn!("[ai] 获取会话缓存失败 req_id={}", context.req_id);
            return;
        };
        let ai_chat_id = chat_session_cache_entry.context.ai_chat_id.clone();
        drop(chat_session_cache_entry);

        let pid = self.chat_session_cache_manager.get_tool_item_id(&context.req_id, call_id);
        let tool_id = self.chat_session_cache_manager.get_tool_id(&context.req_id, call_id).unwrap_or_default();
        let status =
            self.task_registry.event_status_by_call_id(call_id).or_else(|| self.local_shell_registry.event_status_by_call_id(&ai_chat_id, call_id)).unwrap_or_else(|| status_from_result(&result));
        self.task_registry.remove_finished_by_call_id(call_id);
        let call_id = call_id.to_string();
        let base = RespAgentBase::new(context.req_id.clone());
        let run = RespAgentRunBase { run_id: context.run_id.clone() };
        let done = RespAgentToolCallDone { result, status, tool_id, call_id, run, base };
        let event = RespEventAgent::OnToolCallDone(done);
        self.agent_service.handler_agent_event(event, ai_chat_id.to_string(), None, pid, None);
    }
}

fn status_from_result(result: &str) -> i16 {
    if result.starts_with("exit_code=cancelled") {
        return RespAgentToolStatus::Cancelled as i16;
    }
    if result.starts_with("exit_code=killed") {
        return RespAgentToolStatus::Killed as i16;
    }
    RespAgentToolStatus::Done as i16
}
