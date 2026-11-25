use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::oneshot;
use summer::plugin::component::ComponentRef;
use crate::model::dto::cancel_session::CancelSession;
use crate::service::service_known_host::KnownHostService;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};

pub(crate) const CERT_ALGORITHM: &str = "X.509";
pub(crate) const FINGERPRINT_REASON_FIRST_SEEN: i16 = 1;
pub(crate) const FINGERPRINT_REASON_CHANGED: i16 = 2;

pub struct AuthGate {
    pub(crate) answers: Arc<DashMap<String, oneshot::Sender<SshAuthAnswer>>>,
    pub(crate) cancel: Arc<CancelSession>,
    pub(crate) known_host: ComponentRef<KnownHostService>,
}

impl AuthGate {
    pub fn arm(&self, request_id: &str) -> oneshot::Receiver<SshAuthAnswer> {
        let (tx, rx) = oneshot::channel();
        self.answers.insert(request_id.to_string(), tx);
        rx
    }

    pub async fn wait(&self, request_id: &str, rx: oneshot::Receiver<SshAuthAnswer>) -> RdpResult<SshAuthAnswer> {
        let notified = self.cancel.cancel.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if self.cancel.is_cancelled() {
            self.answers.remove(request_id);
            return Err(RdpProtocolError::protocol("RDP open cancelled".to_string()));
        }
        tokio::select! {
            _ = notified => {
                self.answers.remove(request_id);
                Err(RdpProtocolError::protocol("RDP open cancelled".to_string()))
            }
            result = rx => {
                result.map_err(|_| RdpProtocolError::protocol("RDP interaction channel closed".to_string()))
            }
        }
    }
}

pub fn rdp_cert_prompt(stored_algorithm: Option<&str>, stored_sha256: Option<&str>, fingerprint: &str) -> Option<(i16, Option<String>)> {
    match (stored_algorithm, stored_sha256) {
        (Some(algorithm), Some(sha256)) if algorithm == CERT_ALGORITHM && sha256 == fingerprint => None,
        (Some(_), Some(sha256)) => Some((FINGERPRINT_REASON_CHANGED, Some(sha256.to_string()))),
        _ => Some((FINGERPRINT_REASON_FIRST_SEEN, None)),
    }
}
