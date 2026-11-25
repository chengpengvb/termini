use rig::agent::StreamingError;
use summer::tracing::{info, warn};
use crate::module::ai::dto::AiRunContext;
use crate::model::resp::resp_model::ResultCode;
use crate::module::ai::shared::parse_embedded_json;
use crate::module::ai::agent::agent_base::AgentRole;
use rig::completion::{CompletionError, PromptError};
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_build::is_transport_streaming;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentError};
use crate::module::ai::stream::stream_runner::{AiStreamRunner, StreamIdleTimeout};

const HTTP_BAD_REQUEST: u16 = 400;
const HTTP_NOT_IMPLEMENTED: u16 = 501;
const HTTP_SERVER_ERROR_MIN: u16 = 500;
const HTTP_TOO_MANY_REQUESTS: u16 = 429;

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn handle_error(&self, context: &AiRunContext, role: AgentRole, error: StreamingError) -> bool {
        if self.handle_max_turns(context, &error).await {
            return false;
        }
        warn!("[ai] 流出错 req_id={} run_id={} role={} error={}", context.req_id, context.run_id, role.as_name(), error);
        Self::reset_transport_client(context, &error);
        if Self::is_idle_timeout(&error) {
            self.emit_agent_error(context, Some(ResultCode::StreamIdleTimeout.as_i32()), String::new());
            return true;
        }
        if let Some(code) = Self::proxy_result_code(&error) {
            if context.emit.proxy_error {
                self.emit_agent_error(context, Some(code.as_i32()), String::new());
            } else {
                warn!("[ai] 子运行命中权限类错误，交由主运行报告 req_id={} run_id={} code={}", context.req_id, context.run_id, code.as_i32());
            }
        } else {
            self.emit_agent_error(context, None, error.to_string());
        }
        true
    }

    fn is_idle_timeout(error: &StreamingError) -> bool {
        let StreamingError::Completion(CompletionError::RequestError(inner)) = error else {
            return false;
        };
        inner.downcast_ref::<StreamIdleTimeout>().is_some()
    }

    pub(in crate::module::ai::stream) fn is_images_rejected(context: &AiRunContext, error: &StreamingError, produced: bool) -> bool {
        if produced {
            return false;
        }
        if !Self::memory_has_images(context) {
            return false;
        }
        Self::provider_response_status(error) == Some(HTTP_BAD_REQUEST)
    }

    pub(in crate::module::ai::stream) fn is_transient_completion_error(error: &StreamingError) -> bool {
        if !is_transport_streaming(error) {
            return false;
        }
        match Self::provider_response_status(error) {
            Some(status) => status == HTTP_TOO_MANY_REQUESTS || (status >= HTTP_SERVER_ERROR_MIN && status != HTTP_NOT_IMPLEMENTED),
            None => true,
        }
    }

    pub(in crate::module::ai::stream) fn notify_images_ignored(&self, context: &AiRunContext) {
        self.emit_agent_error(context, Some(ResultCode::ImagesUnsupported.as_i32()), String::new());
    }

    fn provider_response_status(error: &StreamingError) -> Option<u16> {
        let status = match error {
            StreamingError::Completion(completion) => completion.provider_response_status(),
            StreamingError::Prompt(prompt) => prompt.provider_response_status(),
        };
        status.map(|status| status.as_u16())
    }

    fn provider_response_body(error: &StreamingError) -> Option<&str> {
        match error {
            StreamingError::Completion(completion) => completion.provider_response_body(),
            StreamingError::Prompt(prompt) => prompt.provider_response_body(),
        }
    }

    fn proxy_result_code(error: &StreamingError) -> Option<ResultCode> {
        let body = Self::provider_response_body(error)?;
        let value = parse_embedded_json::<serde_json::Value>(body)?;
        let code = value.get("error")?.get("code")?.as_i64()? as i32;
        if code == ResultCode::NoAccount.as_i32() {
            Some(ResultCode::NoAccount)
        } else if code == ResultCode::NoBaseUrl.as_i32() {
            Some(ResultCode::NoBaseUrl)
        } else if code == ResultCode::BillingRate.as_i32() {
            Some(ResultCode::BillingRate)
        } else if code == ResultCode::Quota.as_i32() {
            Some(ResultCode::Quota)
        } else if code == ResultCode::Subscription.as_i32() {
            Some(ResultCode::Subscription)
        } else if code == ResultCode::UserDisabled.as_i32() {
            Some(ResultCode::UserDisabled)
        } else {
            None
        }
    }

    fn emit_agent_error(&self, context: &AiRunContext, code: Option<i32>, error: String) {
        let Some(active_chat_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        let ai_chat_id = active_chat_entry.context.ai_chat_id.clone();
        let resp_agent_error = RespEventAgent::OnError(RespAgentError { code, error, base: RespAgentBase::new(context.req_id.clone()) });
        self.agent_service.handler_agent_event(resp_agent_error, ai_chat_id, None, None, None);
    }

    fn reset_transport_client(context: &AiRunContext, error: &StreamingError) {
        let completion_failed = match error {
            StreamingError::Completion(_) => true,
            StreamingError::Prompt(prompt_err) => matches!(prompt_err.as_ref(), PromptError::CompletionError(_)),
        };
        if !completion_failed {
            return;
        }
        ProvideFactory::reset_http_client();
        warn!("[ai] 补全请求失败已重置 provider http client req_id={} run_id={}", context.req_id, context.run_id);
    }

    async fn handle_max_turns(&self, context: &AiRunContext, error: &StreamingError) -> bool {
        let StreamingError::Prompt(prompt_err) = error else {
            return false;
        };
        let PromptError::MaxTurnsError { max_turns, chat_history, .. } = prompt_err.as_ref() else {
            return false;
        };
        info!("[ai] 达到最大迭代 req_id={} run_id={} max_turns={}", context.req_id, context.run_id, max_turns);
        let clean = Self::sanitize_tool_history(chat_history.as_ref().clone());
        *context.memory.lock() = Self::without_addon_messages(clean, &context.addon_messages);
        self.mark_auto_continue(context);
        true
    }
}
