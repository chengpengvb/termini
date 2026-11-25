use std::sync::Arc;
use russh::MethodKind;
use russh::keys::ssh_key;
use russh::client::AuthResult;
use crate::util::id_factory::uuid;
use anyhow::{anyhow, bail, Result};
use russh::keys::PrivateKeyWithHashAlg;
use crate::model::dto::shared_ptr::Shared;
use crate::service::service_base::BaseService;
use crate::model::config_key_pair::ConfigKeyPair;
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
    pub(crate) async fn try_publickey(&self, callback: &SshOpenCallback, call: &mut SshAuthCall<'_, ClientSshHandler>) -> Result<SshAuthAttempt> {
        if call.context.config.read().use_ssh_agent {
            return self.try_ssh_agent(callback, call).await;
        }
        let mut last_key_pair_error: Option<String> = None;
        loop {
            let key_pair_id = match self.resolve_key_pair_id(callback, call, &last_key_pair_error).await? {
                Some(value) => value,
                None => return Ok(SshAuthAttempt::Reselect),
            };
            let key_pair_ptr: Option<Shared<ConfigKeyPair>> = self.key_pair_service().find_by_id(key_pair_id.clone()).await?;
            let Some(key_pair_ptr) = key_pair_ptr else {
                last_key_pair_error = Some("Key pair not found".to_string());
                continue;
            };
            let key_pair = key_pair_ptr.read().clone();
            let private_key_str = key_pair.private_key.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
            let Some(private_key_str) = private_key_str else {
                return Ok(SshAuthAttempt::Rejected("Private key is empty".to_string()));
            };

            let private_key = match Self::decode_private_key_from_context(call.context, private_key_str) {
                Ok(v) => v,
                Err(error) => {
                    let passphrase_error = Some(error.to_string());
                    if !self.prompt_passphrase(callback, call, &passphrase_error).await? {
                        return Ok(SshAuthAttempt::Reselect);
                    }
                    continue;
                }
            };
            let private_key = Arc::new(private_key);
            let cert_str = key_pair.certificate.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
            let result = if cert_str.is_none() {
                callback
                    .wait_until_cancelled_or(|| async {
                        let rsa_hash = call.handle.best_supported_rsa_hash().await.ok().flatten().flatten();
                        let pk = PrivateKeyWithHashAlg::new(private_key.clone(), rsa_hash);
                        call.handle.authenticate_publickey(call.username, pk).await.map_err(|error| anyhow!("SSH authenticate_publickey failed: {}", error))
                    })
                    .await?
            } else {
                let cert = ssh_key::Certificate::from_openssh(cert_str.unwrap()).map_err(|e| anyhow!("Invalid openssh certificate: {}", e))?;
                callback
                    .wait_until_cancelled_or(|| async {
                        call.handle.authenticate_openssh_cert(call.username, private_key.clone(), cert.clone()).await.map_err(|error| anyhow!("SSH authenticate_openssh_cert failed: {}", error))
                    })
                    .await?
            };
            match result {
                AuthResult::Success => {
                    return Ok(SshAuthAttempt::Success(SshAuthMethod::PublicKey));
                }
                AuthResult::Failure { remaining_methods, partial_success: true } => {
                    let remaining: Vec<MethodKind> = remaining_methods.iter().cloned().collect();
                    return Ok(SshAuthAttempt::PartialSuccess { remaining });
                }
                AuthResult::Failure { .. } => {
                    last_key_pair_error = Some("Public key rejected by server".to_string());
                }
            }
        }
    }

    async fn resolve_key_pair_id(&self, callback: &SshOpenCallback, call: &mut SshAuthCall<'_, ClientSshHandler>, last_error: &Option<String>) -> Result<Option<String>> {
        loop {
            let key_pair_id = call.context.config.read().config_key_pair_id.trim().to_string();
            if last_error.is_none() && !key_pair_id.is_empty() {
                return Ok(Some(key_pair_id));
            }
            let request_id = uuid();
            let (step_type, payload) = match last_error.as_ref() {
                Some(message) => {
                    let dto = RetryPrompt { request_id: request_id.clone(), last_error: message.clone() };
                    (RespSshOpenStepType::AuthKeyPairRetry, serde_json::to_value(dto)?)
                }
                None => (RespSshOpenStepType::AuthKeyPairInput, serde_json::to_value(InputPrompt { request_id: request_id.clone() })?),
            };
            let progress = f32::max(callback.current_progress() + 5.0, 66.0);
            let prompt = SshPrompt::new(call.channel_id, &request_id, step_type, call.handler, payload);
            let answer = self.prompt_auth_answer_or_reselect(&prompt, SshAuthAnswer::KeyPairUpdated, progress, call.can_reselect).await?;
            if matches!(answer, SshAuthAnswer::ReselectRequested) {
                return Ok(None);
            }
            let key_pair_id = call.context.config.read().config_key_pair_id.trim().to_string();
            if !key_pair_id.is_empty() {
                return Ok(Some(key_pair_id));
            }
        }
    }

    fn decode_private_key_from_context(context: &ContextSsh, private_key: &str) -> Result<russh::keys::PrivateKey> {
        let guard = context.config.read();
        let passphrase = guard.passphrase.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        russh::keys::decode_secret_key(private_key, passphrase).map_err(|e| anyhow!("Decode secret key failed: {}", e))
    }

    async fn prompt_passphrase(&self, callback: &SshOpenCallback, call: &mut SshAuthCall<'_, ClientSshHandler>, last_error: &Option<String>) -> Result<bool> {
        let request_id = uuid();
        let (step_type, payload) = match last_error.as_ref() {
            Some(message) => {
                let dto = RetryPrompt { request_id: request_id.clone(), last_error: message.clone() };
                (RespSshOpenStepType::AuthPassphraseRetry, serde_json::to_value(dto)?)
            }
            None => (RespSshOpenStepType::AuthPassphraseInput, serde_json::to_value(InputPrompt { request_id: request_id.clone() })?),
        };
        let progress = f32::max(callback.current_progress() + 5.0, 66.0);
        let prompt = SshPrompt::new(call.channel_id, &request_id, step_type, call.handler, payload);
        let answer = self.prompt_auth_answer_or_reselect(&prompt, SshAuthAnswer::PassphraseUpdated, progress, call.can_reselect).await?;
        if matches!(answer, SshAuthAnswer::ReselectRequested) {
            return Ok(false);
        }
        let guard = call.context.config.read();
        let Some(passphrase) = guard.passphrase.clone() else {
            bail!("Passphrase is empty");
        };
        if passphrase.trim().is_empty() {
            bail!("Passphrase is empty");
        }
        Ok(true)
    }
}
