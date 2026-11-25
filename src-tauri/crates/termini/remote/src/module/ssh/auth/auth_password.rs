use russh::MethodKind;
use anyhow::{anyhow, Result};
use russh::client::AuthResult;
use crate::util::id_factory::uuid;
use crate::module::common::dto::ssh_prompt::SshPrompt;
use crate::module::ssh::auth::auth_method::SshAuthMethod;
use crate::module::ssh::auth::auth_method::SshAuthAttempt;
use crate::module::common::dto::ssh_auth_call::SshAuthCall;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::ssh::client_ssh_handler::ClientSshHandler;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::dto::ssh_auth_prompt::{InputPrompt, RetryPrompt};
use crate::model::resp::event::resp_event_ssh_auth::{RespSshOpenStepType, SshOpenCallback};

impl ClientSessionFactory {
    pub(crate) async fn try_password(&self, callback: &SshOpenCallback, call: &mut SshAuthCall<'_, ClientSshHandler>) -> Result<SshAuthAttempt> {
        let mut last_error: Option<String> = None;
        let mut current_password = Self::resolve_password_from_context(call.context)?;
        loop {
            let password = match current_password.clone() {
                Some(value) => value,
                None => match self.prompt_password(callback, call, &last_error).await? {
                    Some(value) => value,
                    None => return Ok(SshAuthAttempt::Reselect),
                },
            };
            let password = password.trim().to_string();
            if password.is_empty() {
                current_password = None;
                last_error = Some("Password is empty".to_string());
                continue;
            }

            let result = callback
                .wait_until_cancelled_or(|| async { call.handle.authenticate_password(call.username, password).await.map_err(|error| anyhow!("SSH authenticate_password failed: {}", error)) })
                .await?;
            match result {
                AuthResult::Success => {
                    return Ok(SshAuthAttempt::Success(SshAuthMethod::Password));
                }
                AuthResult::Failure { remaining_methods, partial_success: true } => {
                    let remaining: Vec<MethodKind> = remaining_methods.iter().cloned().collect();
                    return Ok(SshAuthAttempt::PartialSuccess { remaining });
                }
                AuthResult::Failure { .. } => {
                    last_error = Some("Password rejected by server".to_string());
                    current_password = None;
                }
            }
        }
    }

    fn resolve_password_from_context(context: &ContextSsh) -> Result<Option<String>> {
        let guard = context.config.read();
        let base = guard.get_base()?;
        let Some(password) = base.password else {
            return Ok(None);
        };
        let trimmed = password.trim().to_string();
        if trimmed.is_empty() {
            return Ok(None);
        }
        Ok(Some(trimmed))
    }

    async fn prompt_password(&self, callback: &SshOpenCallback, call: &mut SshAuthCall<'_, ClientSshHandler>, last_error: &Option<String>) -> Result<Option<String>> {
        let request_id = uuid();
        let (step_type, payload) = match last_error.as_ref() {
            Some(message) => {
                let dto = RetryPrompt { request_id: request_id.clone(), last_error: message.clone() };
                (RespSshOpenStepType::AuthPasswordRetry, serde_json::to_value(dto)?)
            }
            None => (RespSshOpenStepType::AuthPasswordInput, serde_json::to_value(InputPrompt { request_id: request_id.clone() })?),
        };
        let progress = f32::max(callback.current_progress() + 5.0, 65.0);

        let prompt = SshPrompt::new(call.channel_id, &request_id, step_type, call.handler, payload);
        let answer = self.prompt_auth_answer_or_reselect(&prompt, SshAuthAnswer::PasswordUpdated, progress, call.can_reselect).await?;
        if matches!(answer, SshAuthAnswer::ReselectRequested) {
            return Ok(None);
        }
        let value = Self::resolve_password_from_context(call.context)?.ok_or_else(|| anyhow!("Password is empty"))?;
        Ok(Some(value))
    }
}
