use std::pin::Pin;
use anyhow::Result;
use serde::Serialize;
use std::future::Future;
use tokio::sync::Notify;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::common::client_terminal::EventHandler;
use crate::common::dto::ssh_channel_kind::SshChannelKind;

#[repr(i16)]
#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(into = "i16")]
pub enum RespSshOpenStepType {
    ProbeStart = 100,
    ProbeTcpConnecting = 101,
    ProbeTcpConnected = 102,
    ProbeNegotiating = 103,
    ProbeCompleted = 104,

    AuthStart = 200,
    AuthUsernameInput = 210,
    AuthUsernameRetry = 211,
    AuthPasswordInput = 212,
    AuthPasswordRetry = 213,
    AuthPassphraseInput = 214,
    AuthPassphraseRetry = 215,
    AuthKeyPairInput = 216,
    AuthKeyPairRetry = 217,
    AuthKeyboardInteractiveInput = 218,
    AuthKeyboardInteractiveRetry = 219,
    AuthHostFingerprintConfirm = 220,
    AuthSshAgentKeySelect = 221,
    AuthMethodSelect = 222,

    SessionOpened = 300,

    ErrorTcpFailed = 500,
    ErrorProbeFailed = 501,
    ErrorAuthFailed = 502,
    ErrorAuthTimeout = 503,
    ErrorChannelFailed = 504,
    ErrorOpenCancelled = 505,
}

impl From<RespSshOpenStepType> for i16 {
    fn from(value: RespSshOpenStepType) -> Self {
        value as i16
    }
}

#[derive(Clone, Serialize)]
pub struct OpenSshStep {
    pub progress: f32,
    pub step_type: RespSshOpenStepType,
    pub data: Option<serde_json::Value>,
}

pub trait SshConnectEvent: Send + Sized + 'static {
    fn on_open_step(step: OpenSshStep) -> Self;
    fn on_auth_banner(_data: Vec<u8>) -> Option<Self> {
        None
    }
    fn on_session_closed() -> Option<Self> {
        None
    }
}

type SshOpenFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

#[derive(Clone)]
pub struct SshOpenCallback {
    kind: SshChannelKind,
    cancelled: Arc<AtomicBool>,
    pub session_id: String,
    current_progress: Arc<Mutex<f32>>,
    pub cancel_notify: Arc<Notify>,
    on_closed: Arc<dyn Fn() -> SshOpenFuture + Send + Sync>,
    on_step: Arc<dyn Fn(OpenSshStep) -> SshOpenFuture + Send + Sync>,
    on_banner: Arc<dyn Fn(Vec<u8>) -> SshOpenFuture + Send + Sync>,
}

impl SshOpenCallback {
    pub fn new<E: SshConnectEvent>(session_id: String, handler: EventHandler<E>, kind: SshChannelKind) -> Self {
        let h_step = handler.clone();
        let h_closed = handler.clone();
        let h_banner = handler;
        Self {
            kind,
            session_id,
            cancel_notify: Arc::new(Notify::new()),
            cancelled: Arc::new(AtomicBool::new(false)),
            on_step: Arc::new(move |step| {
                let e = E::on_open_step(step);
                let h = h_step.clone();
                Box::pin(async move { h(e).await })
            }),
            on_banner: Arc::new(move |data| {
                let mapped = E::on_auth_banner(data);
                let h = h_banner.clone();
                Box::pin(async move {
                    if let Some(e) = mapped {
                        h(e).await?;
                    }
                    Ok(())
                })
            }),
            on_closed: Arc::new(move || {
                let mapped = E::on_session_closed();
                let h = h_closed.clone();
                Box::pin(async move {
                    if let Some(e) = mapped {
                        h(e).await?;
                    }
                    Ok(())
                })
            }),
            current_progress: Arc::new(Mutex::new(0.0)),
        }
    }

    pub fn kind(&self) -> SshChannelKind {
        self.kind
    }

    pub fn interactive_forbidden(&self) -> bool {
        matches!(self.kind, SshChannelKind::Shell)
    }

    pub fn current_progress(&self) -> f32 {
        let guard = self.current_progress.lock().unwrap();
        *guard
    }

    pub async fn emit_open_step(&self, step_type: RespSshOpenStepType, progress: f32, data: Option<serde_json::Value>) {
        {
            let mut guard = self.current_progress.lock().unwrap();
            *guard = progress;
        }
        let step = OpenSshStep { progress, step_type, data };
        let _ = (self.on_step)(step).await;
    }

    pub async fn emit_auth_banner(&self, data: Vec<u8>) {
        let _ = (self.on_banner)(data).await;
    }

    pub async fn emit_session_closed(&self) {
        let _ = (self.on_closed)().await;
    }

    pub fn cancel(&self) {
        if self.cancelled.swap(true, Ordering::AcqRel) {
            return;
        }
        self.cancel_notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn ensure_not_cancelled(&self) -> Result<()> {
        if self.is_cancelled() {
            return Err(anyhow::anyhow!("SSH open cancelled"));
        }
        Ok(())
    }

    pub async fn wait_until_cancelled_or<F, Fut, T>(&self, fut: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        if self.is_cancelled() {
            return Err(anyhow::anyhow!("SSH open cancelled"));
        }
        let fut = fut();
        let cancelled = self.cancel_notify.notified();
        tokio::pin!(fut);
        tokio::pin!(cancelled);
        cancelled.as_mut().enable();
        tokio::select! {
            _ = &mut cancelled => Err(anyhow::anyhow!("SSH open cancelled")),
            result = &mut fut => result,
        }
    }
}
