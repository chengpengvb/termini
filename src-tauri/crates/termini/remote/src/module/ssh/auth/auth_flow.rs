use russh::MethodKind;
use russh::client::Handle;
use russh::client::AuthResult;
use crate::util::id_factory::uuid;
use anyhow::{anyhow, bail, Result};
use crate::module::common::dto::ssh_prompt::SshPrompt;
use crate::module::ssh::auth::auth_method::SshAuthMethod;
use crate::module::ssh::auth::auth_method::SshAuthAttempt;
use crate::module::common::dto::ssh_auth_call::SshAuthCall;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::ssh::client_ssh_handler::ClientSshHandler;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::model::resp::event::resp_event_ssh_auth::{RespSshOpenStepType, SshOpenCallback};
use crate::module::common::dto::ssh_auth_prompt::{InputPrompt, MethodSelectPrompt, RetryPrompt};

impl ClientSessionFactory {
    pub(crate) async fn authenticate(&self, channel_id: &str, handle: &mut Handle<ClientSshHandler>, context: &ContextSsh, callback: &SshOpenCallback) -> Result<SshAuthMethod> {
        callback.ensure_not_cancelled()?;
        let username = self.resolve_username(channel_id, context, callback).await?;
        callback.ensure_not_cancelled()?;
        let username_for_none = username.clone();
        let auth_result = callback.wait_until_cancelled_or(|| async { handle.authenticate_none(username_for_none).await.map_err(|error| anyhow!("SSH authenticate_none failed: {}", error)) }).await?;
        let allowed: Vec<MethodKind> = match auth_result {
            AuthResult::Success => return Ok(SshAuthMethod::None),
            AuthResult::Failure { remaining_methods, .. } => remaining_methods.iter().cloned().collect(),
        };
        loop {
            callback.ensure_not_cancelled()?;
            let preference = self.resolve_auth_preference(channel_id, context, callback, &allowed).await?;
            if let Some(method) = self.authenticate_allowed_methods(username.as_str(), channel_id, context, preference, allowed.clone(), callback, handle).await? {
                return Ok(method);
            }
            context.config.write().preferred_auth_method = None;
        }
    }

    fn supported_methods() -> [MethodKind; 3] {
        [MethodKind::PublicKey, MethodKind::KeyboardInteractive, MethodKind::Password]
    }

    fn method_kind_to_code(kind: &MethodKind) -> Option<i16> {
        SshAuthMethod::from_method_kind(kind).map(|m| m.to_code() as i16)
    }

    fn method_kind_from_code(code: i8) -> Option<MethodKind> {
        SshAuthMethod::from_code(code).and_then(|m| m.to_method_kind())
    }

    async fn resolve_username(&self, channel_id: &str, context: &ContextSsh, callback: &SshOpenCallback) -> Result<String> {
        let mut last_error: Option<String> = None;
        loop {
            callback.ensure_not_cancelled()?;
            if let Some(value) = Self::read_username(context)? {
                return Ok(value);
            }
            let request_id = uuid();
            let (step_type, payload) = match last_error.as_ref() {
                Some(message) => {
                    let dto = RetryPrompt { request_id: request_id.clone(), last_error: message.clone() };
                    (RespSshOpenStepType::AuthUsernameRetry, serde_json::to_value(dto)?)
                }
                None => (RespSshOpenStepType::AuthUsernameInput, serde_json::to_value(InputPrompt { request_id: request_id.clone() })?),
            };
            let prompt = SshPrompt::new(channel_id, &request_id, step_type, callback, payload);
            let _ = self.prompt_and_wait_auth_answer(&prompt, 0.0, 180_000, &[SshAuthAnswer::UsernameUpdated]).await?;
            if let Some(value) = Self::read_username(context)? {
                return Ok(value);
            }
            last_error = Some("Username is empty".to_string());
        }
    }

    fn read_username(context: &ContextSsh) -> Result<Option<String>> {
        let guard = context.config.read();
        let base = guard.get_base()?;
        let Some(username) = base.username else {
            return Ok(None);
        };
        let username = username.trim().to_string();
        if username.is_empty() {
            return Ok(None);
        }
        Ok(Some(username))
    }

    async fn resolve_auth_preference(&self, channel_id: &str, context: &ContextSsh, callback: &SshOpenCallback, allowed: &[MethodKind]) -> Result<Vec<MethodKind>> {
        let supported = Self::supported_methods();
        let intersection: Vec<MethodKind> = supported.iter().filter(|method| allowed.contains(method)).cloned().collect();
        if intersection.is_empty() {
            bail!("No supported authentication methods available");
        }
        if intersection.len() == 1 {
            return Ok(intersection);
        }
        if let Some(chosen) = self.resolve_preferred_from_config(context, &intersection) {
            return Ok(Self::reorder_with_chosen(intersection, chosen));
        }
        let request_id = uuid();
        let progress = f32::max(callback.current_progress() + 5.0, 68.0);

        let default = self.resolve_default_recommendation(context, &intersection);
        let codes: Vec<i16> = intersection.iter().filter_map(Self::method_kind_to_code).collect();
        let payload = MethodSelectPrompt { request_id: request_id.clone(), methods: codes, default };
        let prompt = SshPrompt::new(channel_id, &request_id, RespSshOpenStepType::AuthMethodSelect, callback, serde_json::to_value(payload)?);

        let _ = self.prompt_and_wait_auth_answer(&prompt, progress, 180_000, &[SshAuthAnswer::MethodSelectUpdated]).await?;
        let Some(chosen) = self.resolve_preferred_from_config(context, &intersection) else {
            bail!("Authentication method not selected");
        };
        Ok(Self::reorder_with_chosen(intersection, chosen))
    }

    fn resolve_default_recommendation(&self, context: &ContextSsh, intersection: &[MethodKind]) -> Option<i16> {
        let config = context.config.read();
        let prefers_publickey = !config.config_key_pair_id.trim().is_empty() || config.use_ssh_agent;
        if prefers_publickey && intersection.contains(&MethodKind::PublicKey) {
            return Self::method_kind_to_code(&MethodKind::PublicKey);
        }
        let has_password = match config.get_base() {
            Ok(base) => match base.password {
                Some(password) => !password.trim().is_empty(),
                None => false,
            },
            Err(_) => false,
        };
        if has_password && intersection.contains(&MethodKind::Password) {
            return Self::method_kind_to_code(&MethodKind::Password);
        }
        None
    }

    fn resolve_preferred_from_config(&self, context: &ContextSsh, intersection: &[MethodKind]) -> Option<MethodKind> {
        let snapshot = context.config.read();
        let code = snapshot.preferred_auth_method?;
        let chosen = Self::method_kind_from_code(code)?;
        if intersection.contains(&chosen) {
            Some(chosen)
        } else {
            None
        }
    }

    fn reorder_with_chosen(intersection: Vec<MethodKind>, chosen: MethodKind) -> Vec<MethodKind> {
        let mut preference = vec![chosen.clone()];
        preference.extend(intersection.into_iter().filter(|method| method != &chosen));
        preference
    }

    async fn authenticate_allowed_methods(
        &self,
        username: &str,
        channel_id: &str,
        context: &ContextSsh,
        preference: Vec<MethodKind>,
        mut allowed: Vec<MethodKind>,
        callback: &SshOpenCallback,
        handle: &mut Handle<ClientSshHandler>,
    ) -> Result<Option<SshAuthMethod>> {
        let can_reselect = preference.len() > 1;
        let mut last_error: Option<String> = None;
        let mut tried: Vec<MethodKind> = Vec::new();
        loop {
            callback.ensure_not_cancelled()?;
            let Some(method) = preference.iter().find(|m| allowed.contains(m) && !tried.contains(m)).cloned() else {
                match last_error {
                    Some(message) => bail!(message),
                    None => bail!("No supported authentication methods available"),
                }
            };
            tried.push(method.clone());
            let mut call = SshAuthCall::new(channel_id, handle, username, context, callback, can_reselect);
            let attempt = match method {
                MethodKind::Password => self.try_password(callback, &mut call).await?,
                MethodKind::PublicKey => self.try_publickey(callback, &mut call).await?,
                MethodKind::KeyboardInteractive => self.try_keyboard_interactive(callback, &mut call).await?,
                _ => SshAuthAttempt::Rejected("Unsupported authentication method".to_string()),
            };
            match attempt {
                SshAuthAttempt::Reselect => return Ok(None),
                SshAuthAttempt::Success(m) => return Ok(Some(m)),
                SshAuthAttempt::PartialSuccess { remaining } => {
                    allowed = remaining;
                    tried.clear();
                }
                SshAuthAttempt::Rejected(message) => {
                    last_error = Some(message);
                }
            }
        }
    }
}
