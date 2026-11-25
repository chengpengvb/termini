use russh::MethodKind;
use crate::SshAuthAnswer;
use crate::util::id_factory::uuid;
use anyhow::{anyhow, bail, Result};
use russh::client::KeyboardInteractiveAuthResponse;
use crate::module::common::dto::ssh_prompt::SshPrompt;
use crate::module::ssh::auth::auth_method::SshAuthMethod;
use crate::module::ssh::auth::auth_method::SshAuthAttempt;
use crate::module::common::dto::ssh_auth_call::SshAuthCall;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::ssh::client_ssh_handler::ClientSshHandler;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::dto::ssh_auth_prompt::{InteractivePrompt, InteractiveQuestion};
use crate::model::resp::event::resp_event_ssh_auth::{RespSshOpenStepType, SshOpenCallback};

impl ClientSessionFactory {
    pub(crate) async fn try_keyboard_interactive(&self, callback: &SshOpenCallback, call: &mut SshAuthCall<'_, ClientSshHandler>) -> Result<SshAuthAttempt> {
        let mut already_prompted = false;
        let mut response = callback
            .wait_until_cancelled_or(|| async {
                call.handle.authenticate_keyboard_interactive_start(call.username, None::<String>).await.map_err(|error| anyhow!("SSH keyboard-interactive start failed: {}", error))
            })
            .await?;
        loop {
            callback.ensure_not_cancelled()?;
            match response {
                KeyboardInteractiveAuthResponse::Success => {
                    return Ok(SshAuthAttempt::Success(SshAuthMethod::KeyboardInteractive));
                }
                KeyboardInteractiveAuthResponse::Failure { partial_success: true, remaining_methods, .. } => {
                    let remaining: Vec<MethodKind> = remaining_methods.iter().cloned().collect();
                    return Ok(SshAuthAttempt::PartialSuccess { remaining });
                }
                KeyboardInteractiveAuthResponse::Failure { .. } => {
                    return Ok(SshAuthAttempt::Rejected("Keyboard-interactive authentication rejected by server".to_string()));
                }
                KeyboardInteractiveAuthResponse::InfoRequest { name, instructions, prompts } => {
                    let expected = prompts.len();
                    if expected == 0 {
                        response = callback
                            .wait_until_cancelled_or(|| async {
                                call.handle.authenticate_keyboard_interactive_respond(Vec::new()).await.map_err(|error| anyhow!("SSH keyboard-interactive respond failed: {}", error))
                            })
                            .await?;
                        continue;
                    }

                    let request_id = uuid();
                    let questions: Vec<InteractiveQuestion> = prompts.iter().map(|p| InteractiveQuestion { text: p.prompt.clone(), echo: p.echo }).collect();
                    let payload = InteractivePrompt { request_id: request_id.clone(), name, instruction: instructions, questions };
                    let step_type = if already_prompted { RespSshOpenStepType::AuthKeyboardInteractiveRetry } else { RespSshOpenStepType::AuthKeyboardInteractiveInput };
                    already_prompted = true;

                    let progress = f32::max(callback.current_progress() + 5.0, 65.0);
                    let prompt = SshPrompt::new(call.channel_id, &request_id, step_type, call.handler, serde_json::to_value(payload)?);
                    let answer = self.prompt_auth_answer_or_reselect(&prompt, SshAuthAnswer::KeyboardInteractiveUpdated, progress, call.can_reselect).await?;
                    if matches!(answer, SshAuthAnswer::ReselectRequested) {
                        return Ok(SshAuthAttempt::Reselect);
                    }

                    let answers = Self::read_keyboard_answers(call.context)?;
                    if answers.len() != expected {
                        bail!("Keyboard-interactive answer count mismatch");
                    }
                    response = callback
                        .wait_until_cancelled_or(|| async {
                            call.handle.authenticate_keyboard_interactive_respond(answers).await.map_err(|error| anyhow!("SSH keyboard-interactive respond failed: {}", error))
                        })
                        .await?;
                }
            }
        }
    }

    fn read_keyboard_answers(context: &ContextSsh) -> Result<Vec<String>> {
        let guard = context.config.read();
        let Some(answers) = guard.get_keyboard_interactive_answers()? else {
            return Err(anyhow!("Keyboard-interactive answers missing"));
        };
        Ok(answers)
    }
}
