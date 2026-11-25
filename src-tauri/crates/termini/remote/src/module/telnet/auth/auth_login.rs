use std::sync::Arc;
use crate::module::telnet::client_telnet::ClientTelnet;
use crate::module::common::client_terminal::ClientTerminal;
use crate::module::common::context::context_telnet::ContextTelnet;
use crate::module::telnet::prompt::prompt_pattern::TelnetPromptPattern;
use crate::module::telnet::prompt::prompt_text::{normalize_linefeeds, strip_ansi_escape_codes};

const AUTH_BUFFER_CAP: usize = 4096;
const AUTH_MATCH_WINDOW: usize = 512;
const AUTH_MAX_TRY_TIMES: i16 = 30;
const CREDENTIAL_LINE_END: u8 = 0x0D;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    None,
    Error,
    UsernameSent,
    PasswordSent,
    LoginSuccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthLoginType {
    NONE,
    Error, //用户名或者密码缺失的情况无需进行登录尝试，直接返回错误即可
    OnlyUsername(String),
    UsernameAndPassword(String, String),
}

pub struct TelnetAuthMeta {
    pub try_times: i16,
    pub echo: String,
    pub auth_state: AuthState,
    pub login_type: AuthLoginType,
    pub pattern: Option<TelnetPromptPattern>,
}

impl TelnetAuthMeta {
    pub fn push_echo(&mut self, data: &[u8]) {
        self.echo.push_str(&String::from_utf8_lossy(data));
        if self.echo.len() <= AUTH_BUFFER_CAP {
            return;
        }
        let cut = self.echo.len() - AUTH_BUFFER_CAP;
        let boundary = (cut..self.echo.len()).find(|index| self.echo.is_char_boundary(*index)).unwrap_or(self.echo.len());
        self.echo = self.echo[boundary..].to_string();
    }

    pub fn echo_tail(&self) -> String {
        let start = self.echo.len().saturating_sub(AUTH_MATCH_WINDOW);
        let boundary = (start..self.echo.len()).find(|index| self.echo.is_char_boundary(*index)).unwrap_or(0);
        normalize_linefeeds(&strip_ansi_escape_codes(&self.echo[boundary..]))
    }

    pub fn clear_echo(&mut self) {
        self.echo.clear();
    }

    pub fn is_credential_sent(&self) -> bool {
        matches!(self.auth_state, AuthState::PasswordSent | AuthState::LoginSuccess)
    }
}

impl ClientTelnet {
    pub async fn try_login(&self, data: Vec<u8>) {
        let mut auth_meta = self.auth_meta.lock().await;
        if auth_meta.auth_state == AuthState::LoginSuccess || auth_meta.auth_state == AuthState::Error || auth_meta.login_type == AuthLoginType::Error {
            return;
        }
        if auth_meta.try_times >= AUTH_MAX_TRY_TIMES {
            auth_meta.auth_state = AuthState::Error;
            return;
        }
        auth_meta.try_times += 1;
        auth_meta.push_echo(&data);

        let context = {
            let guard = self.context.lock().await;
            let Some(ctx) = guard.as_ref() else {
                return;
            };
            Arc::clone(ctx)
        };
        if auth_meta.pattern.is_none() {
            auth_meta.pattern = Some(TelnetPromptPattern::from_config(&context.config.read()));
        }
        let Some(pattern) = auth_meta.pattern.clone() else {
            return;
        };

        let login_type = match auth_meta.login_type.clone() {
            AuthLoginType::NONE => {
                let resolved = self.init_auth_login_type(&context);
                auth_meta.login_type = resolved.clone();
                resolved
            }
            other => other,
        };
        let echo = auth_meta.echo_tail();
        match login_type {
            AuthLoginType::OnlyUsername(username) => {
                self.advance_only_username(&mut auth_meta, &pattern, &echo, &username).await;
            }
            AuthLoginType::UsernameAndPassword(username, password) => {
                self.advance_username_and_password(&mut auth_meta, &pattern, &echo, &username, &password).await;
            }
            _ => {
                auth_meta.login_type = AuthLoginType::Error;
            }
        }
    }

    fn format_credential_line(value: &str) -> String {
        let mut line = String::with_capacity(value.len() + 1);
        line.push_str(value);
        line.push(char::from(CREDENTIAL_LINE_END));
        line
    }

    async fn send_credential(&self, auth_meta: &mut TelnetAuthMeta, value: &str, next_state: AuthState) {
        match self.write(Self::format_credential_line(value)).await {
            Ok(_) => {
                auth_meta.auth_state = next_state;
                auth_meta.clear_echo();
            }
            Err(_) => {
                auth_meta.login_type = AuthLoginType::Error;
            }
        }
    }

    async fn advance_only_username(&self, auth_meta: &mut TelnetAuthMeta, pattern: &TelnetPromptPattern, echo: &str, username: &str) {
        match auth_meta.auth_state {
            AuthState::None => {
                if pattern.is_username_prompt(echo) {
                    self.send_credential(auth_meta, username, AuthState::UsernameSent).await;
                }
            }
            AuthState::UsernameSent => {
                if pattern.is_device_prompt(echo) {
                    auth_meta.auth_state = AuthState::LoginSuccess;
                } else if pattern.is_username_prompt(echo) {
                    self.send_credential(auth_meta, username, AuthState::UsernameSent).await;
                }
            }
            _ => {}
        }
    }

    async fn advance_username_and_password(&self, auth_meta: &mut TelnetAuthMeta, pattern: &TelnetPromptPattern, echo: &str, username: &str, password: &str) {
        match auth_meta.auth_state {
            AuthState::None => {
                if pattern.is_username_prompt(echo) {
                    self.send_credential(auth_meta, username, AuthState::UsernameSent).await;
                } else if pattern.is_password_prompt(echo) {
                    self.send_credential(auth_meta, password, AuthState::PasswordSent).await;
                }
            }
            AuthState::UsernameSent => {
                if pattern.is_password_prompt(echo) {
                    self.send_credential(auth_meta, password, AuthState::PasswordSent).await;
                } else if pattern.is_device_prompt(echo) {
                    auth_meta.auth_state = AuthState::LoginSuccess;
                } else if pattern.is_username_prompt(echo) {
                    self.send_credential(auth_meta, username, AuthState::UsernameSent).await;
                }
            }
            AuthState::PasswordSent => {
                if pattern.is_device_prompt(echo) {
                    auth_meta.auth_state = AuthState::LoginSuccess;
                } else if pattern.is_username_prompt(echo) {
                    auth_meta.auth_state = AuthState::None;
                    auth_meta.clear_echo();
                }
            }
            _ => {}
        }
    }

    fn resolve_credential(value: Option<String>) -> Option<String> {
        match value {
            Some(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            None => None,
        }
    }

    fn init_auth_login_type(&self, context: &ContextTelnet) -> AuthLoginType {
        let Some(base) = context.config.read().get_base().ok() else {
            return AuthLoginType::Error;
        };
        let username = Self::resolve_credential(base.username.clone());
        let password = Self::resolve_credential(base.password.clone());
        match (username, password) {
            (Some(u), None) => AuthLoginType::OnlyUsername(u),
            (Some(u), Some(p)) => AuthLoginType::UsernameAndPassword(u, p),
            _ => AuthLoginType::Error,
        }
    }
}
