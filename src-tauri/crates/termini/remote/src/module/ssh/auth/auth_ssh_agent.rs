use russh::MethodKind;
use russh::keys::HashAlg;
use russh::client::Handle;
use russh::client::AuthResult;
use crate::util::id_factory::uuid;
use anyhow::{anyhow, bail, Result};
use russh::keys::agent::AgentIdentity;
use crate::module::common::dto::ssh_prompt::SshPrompt;
use crate::module::ssh::auth::auth_method::SshAuthMethod;
use crate::module::ssh::auth::auth_method::SshAuthAttempt;
use crate::module::common::dto::ssh_auth_call::SshAuthCall;
use russh::keys::agent::client::{AgentClient, AgentStream};
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::ssh::client_ssh_handler::ClientSshHandler;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::dto::ssh_auth_prompt::{AgentKeyEntry, AgentKeyPrompt};
use crate::model::resp::event::resp_event_ssh_auth::{RespSshOpenStepType, SshOpenCallback};

type DynAgentClient = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

impl ClientSessionFactory {
    pub(crate) async fn try_ssh_agent(&self, callback: &SshOpenCallback, call: &mut SshAuthCall<'_, ClientSshHandler>) -> Result<SshAuthAttempt> {
        let (mut agent, agent_type) = Self::connect_agent().await?;
        let identities = agent.request_identities().await.map_err(|e| anyhow!("SSH agent request identities failed: {}", e))?;
        if identities.is_empty() {
            return Ok(SshAuthAttempt::Rejected("SSH agent has no identities".to_string()));
        }
        let selected = if identities.len() == 1 {
            identities[0].clone()
        } else {
            match self.prompt_agent_key_select(callback, call, agent_type, &identities).await? {
                Some(found) => found,
                None => return Ok(SshAuthAttempt::Reselect),
            }
        };
        self.try_agent_identity(call.username, selected, callback, &mut agent, call.handle).await
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn connect_agent() -> Result<(DynAgentClient, &'static str)> {
        let agent = AgentClient::<tokio::net::UnixStream>::connect_env().await.map_err(|e| anyhow!("SSH agent connect env failed: {}", e))?;
        Ok((agent.dynamic(), "openssh"))
    }

    #[cfg(target_os = "windows")]
    async fn connect_agent() -> Result<(DynAgentClient, &'static str)> {
        if let Ok(mut agent) = AgentClient::connect_pageant().await {
            if let Ok(identities) = agent.request_identities().await {
                if !identities.is_empty() {
                    return Ok((agent.dynamic(), "pageant"));
                }
            }
        }
        let agent = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await.map_err(|e| anyhow!("SSH agent connect named pipe failed: {}", e))?;
        Ok((agent.dynamic(), "openssh"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    async fn connect_agent() -> Result<(DynAgentClient, &'static str)> {
        Err(anyhow!("SSH agent is not supported on this platform"))
    }

    async fn prompt_agent_key_select(
        &self,
        callback: &SshOpenCallback,
        call: &mut SshAuthCall<'_, ClientSshHandler>,
        agent_type: &'static str,
        identities: &[AgentIdentity],
    ) -> Result<Option<AgentIdentity>> {
        let keys: Vec<AgentKeyEntry> = identities
            .iter()
            .map(|identity| {
                let public_key = identity.public_key();
                let fingerprint = public_key.fingerprint(HashAlg::Sha256).to_string();
                AgentKeyEntry { key_id: fingerprint.clone(), comment: identity.comment().to_string(), algorithm: public_key.algorithm().to_string(), fingerprint }
            })
            .collect();
        loop {
            let request_id = uuid();
            let progress = f32::max(callback.current_progress() + 5.0, 70.0);
            let payload = AgentKeyPrompt { request_id: request_id.clone(), agent_type: agent_type.to_string(), keys: keys.clone() };
            let prompt = SshPrompt::new(call.channel_id, &request_id, RespSshOpenStepType::AuthSshAgentKeySelect, call.handler, serde_json::to_value(payload)?);
            let answer = self.prompt_auth_answer_or_reselect(&prompt, SshAuthAnswer::AgentKeySelectUpdated, progress, call.can_reselect).await?;
            if matches!(answer, SshAuthAnswer::ReselectRequested) {
                return Ok(None);
            }
            let selected_key_id = Self::read_agent_key_id(call.context)?;
            if let Some(found) = identities.iter().find(|identity| Self::identity_key_id(identity).as_deref() == Some(selected_key_id.as_str())).cloned() {
                return Ok(Some(found));
            }
        }
    }

    fn read_agent_key_id(context: &ContextSsh) -> Result<String> {
        let guard = context.config.read();
        let Some(key_id) = guard.agent_key_id.clone() else {
            bail!("Agent key id is empty");
        };
        let trimmed = key_id.trim().to_string();
        if trimmed.is_empty() {
            bail!("Agent key id is empty");
        }
        Ok(trimmed)
    }

    fn identity_key_id(identity: &AgentIdentity) -> Option<String> {
        Some(identity.public_key().fingerprint(HashAlg::Sha256).to_string())
    }

    async fn try_agent_identity(
        &self,
        username: &str,
        identity: AgentIdentity,
        callback: &SshOpenCallback,
        agent: &mut DynAgentClient,
        handle: &mut Handle<ClientSshHandler>,
    ) -> Result<SshAuthAttempt> {
        match identity {
            AgentIdentity::PublicKey { key, .. } => {
                let result = callback
                    .wait_until_cancelled_or(|| async {
                        let rsa_hash = handle.best_supported_rsa_hash().await.ok().flatten().flatten();
                        handle.authenticate_publickey_with(username, key, rsa_hash, agent).await.map_err(|error| anyhow!("SSH agent auth failed: {:?}", error))
                    })
                    .await?;
                Ok(Self::parse_auth_result(result, SshAuthMethod::PublicKey))
            }
            AgentIdentity::Certificate { certificate, .. } => {
                let result = callback
                    .wait_until_cancelled_or(|| async {
                        let rsa_hash = handle.best_supported_rsa_hash().await.ok().flatten().flatten();
                        handle.authenticate_certificate_with(username, certificate, rsa_hash, agent).await.map_err(|error| anyhow!("SSH agent auth failed: {:?}", error))
                    })
                    .await?;
                Ok(Self::parse_auth_result(result, SshAuthMethod::PublicKey))
            }
        }
    }

    fn parse_auth_result(result: AuthResult, success_method: SshAuthMethod) -> SshAuthAttempt {
        match result {
            AuthResult::Success => SshAuthAttempt::Success(success_method),
            AuthResult::Failure { partial_success: true, remaining_methods, .. } => {
                let remaining: Vec<MethodKind> = remaining_methods.iter().cloned().collect();
                SshAuthAttempt::PartialSuccess { remaining }
            }
            AuthResult::Failure { .. } => SshAuthAttempt::Rejected("Authentication rejected by server".to_string()),
        }
    }
}
