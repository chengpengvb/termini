use serde::Serialize;

#[repr(i16)]
#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(into = "i16")]
pub enum RespTelnetOpenStepType {
    ProbeStart = 100,
    ProbeTcpConnecting = 101,
    ProbeTcpConnected = 102,
    ProbeNegotiating = 103,
    ProbeCompleted = 104,
    SessionOpened = 200,
    ErrorTcpFailed = 300,
    ErrorProbeFailed = 301,
}

impl From<RespTelnetOpenStepType> for i16 {
    fn from(value: RespTelnetOpenStepType) -> Self {
        value as i16
    }
}

#[derive(Clone, Serialize)]
pub struct OpenTelnetStep {
    pub progress: f32, //0~100 总体进度
    pub step_type: RespTelnetOpenStepType,
    pub data: Option<serde_json::Value>, //该步骤附带的结构化数据；失败步骤在此携带错误信息，成功步骤为 null
}

pub enum RespEventTelnet {
    OnOpen,
    OnClose,
    OnError(String),
    OnData(Vec<u8>),
    OnOpenStep(OpenTelnetStep),
    OnDeviceOsType(i16),
}

impl RespEventTelnet {
    pub fn to_type(&self) -> i16 {
        match self {
            RespEventTelnet::OnOpen => 10,
            RespEventTelnet::OnClose => 11,
            RespEventTelnet::OnError(_) => 12,
            RespEventTelnet::OnData(_) => 13,
            RespEventTelnet::OnOpenStep(_) => 14,
            RespEventTelnet::OnDeviceOsType(_) => 15,
        }
    }
}
