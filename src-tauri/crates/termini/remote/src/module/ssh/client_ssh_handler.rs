use russh::client;
use russh::Channel;
use std::sync::Arc;
use dashmap::DashMap;
use russh::client::Msg;
use russh::keys::HashAlg;
use summer::tracing::warn;
use tokio::net::TcpStream;
use tokio::time::Duration;
use russh::client::Handler;
use russh::keys::PublicKey;
use anyhow::{anyhow, Result};
use russh::ChannelOpenFailure;
use crate::util::id_factory::uuid;
use russh::client::DisconnectReason;
use russh::client::ChannelOpenHandle;
use summer::plugin::component::ComponentRef;
use crate::module::ssh_forward::pump::forward_pump;
use crate::service::service_known_host::KnownHostService;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::common::dto::ssh_session_state::SshAuthInbox;
use crate::module::common::dto::ssh_auth_prompt::FingerprintPrompt;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::model::resp::event::resp_event_ssh_auth::{RespSshOpenStepType, SshOpenCallback};

const FINGERPRINT_REASON_CHANGED: i16 = 2;
const FINGERPRINT_REASON_FIRST_SEEN: i16 = 1;
const PROMPT_FINGERPRINT_TIMEOUT_MS: i64 = 180_000;

pub struct ClientSshHandler {
    pub port: u16,
    pub host: String,
    pub code: Option<i16>,
    pub channel_id: String,
    pub context: ContextSsh,
    pub inbox: Arc<SshAuthInbox>,
    pub callback: SshOpenCallback,
    pub factory: ClientSessionFactory,
    pub known_host_service: ComponentRef<KnownHostService>,
    pub auth_requests: Arc<DashMap<String, Arc<SshAuthInbox>>>,
}

impl ClientSshHandler {
    pub fn new(
        port: u16,
        host: String,
        channel_id: String,
        context: ContextSsh,
        trust_code: Option<i16>,
        inbox: Arc<SshAuthInbox>,
        callback: SshOpenCallback,
        known_host_service: ComponentRef<KnownHostService>,
        auth_requests: Arc<DashMap<String, Arc<SshAuthInbox>>>,
        factory: ClientSessionFactory,
    ) -> Self {
        Self { channel_id, host, port, context, callback, inbox, auth_requests, known_host_service, factory, code: trust_code }
    }

    async fn emit_open_step(&self, step_type: RespSshOpenStepType, progress: f32, data: Option<serde_json::Value>) {
        self.callback.emit_open_step(step_type, progress, data).await;
    }

    async fn wait_fingerprint_trust(&self, request_id: &str) -> Result<SshAuthAnswer> {
        let expected = [SshAuthAnswer::HostFingerprintReject, SshAuthAnswer::HostFingerprintTrustOnce, SshAuthAnswer::HostFingerprintTrustAlways];
        loop {
            if self.callback.is_cancelled() {
                return Err(anyhow!("SSH open cancelled"));
            }
            let notified = self.inbox.notify.notified();
            let cancelled = self.callback.cancel_notify.notified();
            tokio::pin!(notified);
            tokio::pin!(cancelled);
            notified.as_mut().enable();
            cancelled.as_mut().enable();

            if let Some(value) = {
                let mut guard = self.inbox.queue.lock().await;
                let pos = guard.iter().position(|entry| entry.request_id == request_id && expected.contains(&entry.answer));
                match pos {
                    Some(index) => guard.remove(index).map(|entry| entry.answer),
                    None => None,
                }
            } {
                return Ok(value);
            }

            let sleep = tokio::time::sleep(Duration::from_millis(PROMPT_FINGERPRINT_TIMEOUT_MS as u64));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut notified => {}
                _ = &mut cancelled => {
                    return Err(anyhow!("SSH open cancelled"));
                }
                _ = &mut sleep => {
                    return Err(anyhow!("SSH host fingerprint prompt timed out"));
                }
            }
        }
    }
}

impl Handler for ClientSshHandler {
    type Error = russh::Error;

    async fn disconnected(&mut self, reason: DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        self.factory.purge_closed().await;
        match reason {
            DisconnectReason::ReceivedDisconnect(_) => Ok(()),
            DisconnectReason::Error(error) => Err(error),
        }
    }

    async fn auth_banner(&mut self, banner: &str, _session: &mut client::Session) -> Result<(), Self::Error> {
        self.callback.emit_auth_banner(banner.as_bytes().to_vec()).await;
        Ok(())
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let Some(sink) = self.factory.lookup_forward(connected_address, connected_port as u16) else {
            reply.reject(ChannelOpenFailure::AdministrativelyProhibited).await;
            return Ok(());
        };
        reply.accept().await;
        let target = format!("{}:{}", sink.target_host, sink.target_port);
        let conns = sink.conns.clone();
        let task = tokio::spawn(async move {
            let tcp = match TcpStream::connect(&target).await {
                Ok(tcp) => tcp,
                Err(error) => {
                    warn!("[ssh] forward 远端连接目标 {} 失败: {}", target, error);
                    return;
                }
            };
            forward_pump::pump(channel, tcp).await;
        });
        conns.lock().push(task);
        Ok(())
    }

    async fn check_server_key(&mut self, server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        let algorithm = server_public_key.algorithm().to_string();
        let sha256 = server_public_key.fingerprint(HashAlg::Sha256).to_string();

        let stored = self.known_host_service.find_by_host_port(&self.host, self.port).map(|ptr| ptr.read().clone());
        match stored {
            Some(prev) if prev.algorithm == algorithm && prev.sha256 == sha256 => Ok(true),
            prev => {
                let Some(code) = self.code else {
                    if self.callback.interactive_forbidden() {
                        return Ok(false);
                    }
                    let request_id = uuid();
                    let previous_sha256 = prev.as_ref().map(|item| item.sha256.clone());
                    let reason = if prev.is_some() { FINGERPRINT_REASON_CHANGED } else { FINGERPRINT_REASON_FIRST_SEEN };
                    let payload =
                        FingerprintPrompt { request_id: request_id.clone(), port: self.port, reason, host: self.host.clone(), sha256: sha256.clone(), algorithm: algorithm.clone(), previous_sha256 };
                    let data = serde_json::to_value(payload).map_err(|_| russh::Error::Inconsistent)?;
                    self.auth_requests.insert(request_id.clone(), self.inbox.clone());
                    self.emit_open_step(RespSshOpenStepType::AuthHostFingerprintConfirm, 55.0, Some(data)).await;
                    let waited = self.wait_fingerprint_trust(&request_id).await;
                    self.auth_requests.remove(&request_id);
                    let trust = match waited {
                        Ok(v) => v,
                        Err(_) => {
                            if self.callback.is_cancelled() {
                                return Err(russh::Error::Disconnect);
                            }
                            return Ok(false);
                        }
                    };
                    return match trust {
                        SshAuthAnswer::HostFingerprintReject => Ok(false),
                        SshAuthAnswer::HostFingerprintTrustOnce => Ok(true),
                        SshAuthAnswer::HostFingerprintTrustAlways => {
                            let _ = self.known_host_service.upsert_trusted(self.host.clone(), self.port, algorithm, sha256).await;
                            Ok(true)
                        }
                        _ => Ok(false),
                    };
                };
                let trust = SshAuthAnswer::from_code(code);
                match trust {
                    Some(SshAuthAnswer::HostFingerprintReject) => Ok(false),
                    Some(SshAuthAnswer::HostFingerprintTrustOnce) => Ok(true),
                    Some(SshAuthAnswer::HostFingerprintTrustAlways) => {
                        let _ = self.known_host_service.upsert_trusted(self.host.clone(), self.port, algorithm, sha256).await;
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
        }
    }
}
