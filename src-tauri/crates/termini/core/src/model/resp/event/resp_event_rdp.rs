use bytes::Bytes;
use serde::Serialize;

#[repr(i16)]
#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(into = "i16")]
pub enum RespRdpOpenStepType {
    TcpConnecting = 100,
    TcpConnected = 101,
    Negotiating = 102,
    TlsHandshaking = 103,
    TlsEstablished = 104,
    Authenticating = 105,
    Authenticated = 106,
    McsConnecting = 107,
    McsConnected = 108,
    Licensing = 109,
    CapabilitiesExchanging = 110,
    Activating = 111,
    AuthUsernameInput = 210,
    AuthUsernameRetry = 211,
    AuthPasswordInput = 212,
    AuthPasswordRetry = 213,
    CertFingerprintConfirm = 221,
    SessionOpened = 200,
    ErrorTcpFailed = 300,
    ErrorNegotiateFailed = 301,
    ErrorTlsFailed = 302,
    ErrorAuthFailed = 303,
    ErrorMcsFailed = 304,
    ErrorLicenseFailed = 305,
    ErrorCapabilitiesFailed = 306,
    ErrorActivateFailed = 307,
}

impl From<RespRdpOpenStepType> for i16 {
    fn from(value: RespRdpOpenStepType) -> Self {
        value as i16
    }
}

#[derive(Clone, Serialize)]
pub struct OpenRdpStep {
    pub progress: f32,
    pub step_type: RespRdpOpenStepType,
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Serialize)]
pub struct RespRdpDesktopSize {
    pub width: u16,
    pub height: u16,
}

pub enum RespEventRdp {
    OnOpen,
    OnClose,
    OnError(String),
    OnFrame(Bytes),
    OnOpenStep(OpenRdpStep),
    OnCursor(Bytes),
    OnDesktopResize(RespRdpDesktopSize),
    OnNetwork(u8),
}

impl RespEventRdp {
    pub fn to_type(&self) -> i16 {
        match self {
            RespEventRdp::OnOpen => 80,
            RespEventRdp::OnClose => 81,
            RespEventRdp::OnError(_) => 82,
            RespEventRdp::OnFrame(_) => 83,
            RespEventRdp::OnOpenStep(_) => 84,
            RespEventRdp::OnCursor(_) => 85,
            RespEventRdp::OnDesktopResize(_) => 86,
            RespEventRdp::OnNetwork(_) => 87,
        }
    }
}
