use crate::util::id_factory::uuid;
use crate::module::rdp::auth::auth_gate::AuthGate;
use crate::module::common::client_terminal::EventHandler;
use crate::module::common::context::context_rdp::ContextRdp;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::common::dto::ssh_auth_prompt::{InputPrompt, RetryPrompt};
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};
use crate::model::resp::event::resp_event_rdp::{RespEventRdp, RespRdpOpenStepType};

impl RdpSessionFactory {
    pub(crate) async fn resolve_credentials(&self, context: &ContextRdp, gate: &AuthGate, handler: &EventHandler<RespEventRdp>) -> RdpResult<()> {
        while config_username(context).is_none() {
            self.prompt_input(gate, handler, RespRdpOpenStepType::AuthUsernameInput).await?;
        }
        while config_password(context).is_none() {
            self.prompt_input(gate, handler, RespRdpOpenStepType::AuthPasswordInput).await?;
        }
        Ok(())
    }

    async fn prompt_input(&self, gate: &AuthGate, handler: &EventHandler<RespEventRdp>, step_type: RespRdpOpenStepType) -> RdpResult<()> {
        let request_id = uuid();
        let payload = InputPrompt { request_id: request_id.clone() };
        let data = serde_json::to_value(payload).map_err(|error| RdpProtocolError::encoding(format!("failed to serialize login prompt: {}", error)))?;
        let rx = gate.arm(&request_id);
        self.emit_open_step(handler, step_type, 38.0, Some(data)).await;
        gate.wait(&request_id, rx).await?;
        Ok(())
    }

    pub(crate) async fn prompt_password_retry(&self, gate: &AuthGate, handler: &EventHandler<RespEventRdp>, last_error: &str) -> RdpResult<()> {
        let request_id = uuid();
        let payload = RetryPrompt { request_id: request_id.clone(), last_error: last_error.to_string() };
        let data = serde_json::to_value(payload).map_err(|error| RdpProtocolError::encoding(format!("failed to serialize login prompt: {}", error)))?;
        let rx = gate.arm(&request_id);
        self.emit_open_step(handler, RespRdpOpenStepType::AuthPasswordRetry, 38.0, Some(data)).await;
        gate.wait(&request_id, rx).await?;
        Ok(())
    }
}

pub(crate) fn config_username(context: &ContextRdp) -> Option<String> {
    let config = context.config.read().clone();
    config.get_base().ok()?.username.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

pub(crate) fn config_password(context: &ContextRdp) -> Option<String> {
    let config = context.config.read().clone();
    config.get_base().ok()?.password.filter(|value| !value.is_empty())
}
