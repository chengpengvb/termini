pub enum ReqWsEventShell {
    Open = 0,
    Close = 1,
    Write = 2,
    Resize = 3,
    Snapshot = 4,
}

impl ReqWsEventShell {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventShell> {
        match r#type {
            0 => Some(ReqWsEventShell::Open),
            1 => Some(ReqWsEventShell::Close),
            2 => Some(ReqWsEventShell::Write),
            3 => Some(ReqWsEventShell::Resize),
            4 => Some(ReqWsEventShell::Snapshot),
            _ => None,
        }
    }
}

pub enum ReqWsEventTelnet {
    Open = 10,
    Close = 11,
    Write = 12,
    Resize = 13,
}

impl ReqWsEventTelnet {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventTelnet> {
        match r#type {
            10 => Some(ReqWsEventTelnet::Open),
            11 => Some(ReqWsEventTelnet::Close),
            12 => Some(ReqWsEventTelnet::Write),
            13 => Some(ReqWsEventTelnet::Resize),
            _ => None,
        }
    }
}

pub enum ReqWsEventSsh {
    Open = 20,
    Close = 21,
    Write = 22,
    Resize = 23,
}

impl ReqWsEventSsh {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventSsh> {
        match r#type {
            20 => Some(ReqWsEventSsh::Open),
            21 => Some(ReqWsEventSsh::Close),
            22 => Some(ReqWsEventSsh::Write),
            23 => Some(ReqWsEventSsh::Resize),
            _ => None,
        }
    }
}

pub enum ReqWsEventSshAuth {
    AuthAnswer = 30,
}

impl ReqWsEventSshAuth {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventSshAuth> {
        match r#type {
            30 => Some(ReqWsEventSshAuth::AuthAnswer),
            _ => None,
        }
    }
}

pub enum ReqWsEventSftp {
    Open = 40,
    Close = 41,
    HomeDir = 42,
    ListDir = 43,
    Stat = 44,
    Rename = 45,
    Chmod = 46,
    Create = 47,
    Delete = 48,
    Transfer = 49,
    StatProperties = 50,
    TransferAnswer = 51,
    FileOpen = 52,
    ExportToTemp = 53,
    TransferCancel = 54,
    DeleteCancel = 55,
}

impl ReqWsEventSftp {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventSftp> {
        match r#type {
            40 => Some(ReqWsEventSftp::Open),
            41 => Some(ReqWsEventSftp::Close),
            42 => Some(ReqWsEventSftp::HomeDir),
            43 => Some(ReqWsEventSftp::ListDir),
            44 => Some(ReqWsEventSftp::Stat),
            45 => Some(ReqWsEventSftp::Rename),
            46 => Some(ReqWsEventSftp::Chmod),
            47 => Some(ReqWsEventSftp::Create),
            48 => Some(ReqWsEventSftp::Delete),
            49 => Some(ReqWsEventSftp::Transfer),
            50 => Some(ReqWsEventSftp::StatProperties),
            51 => Some(ReqWsEventSftp::TransferAnswer),
            52 => Some(ReqWsEventSftp::FileOpen),
            53 => Some(ReqWsEventSftp::ExportToTemp),
            54 => Some(ReqWsEventSftp::TransferCancel),
            55 => Some(ReqWsEventSftp::DeleteCancel),
            _ => None,
        }
    }
}

pub enum ReqWsEventAgent {
    Chat = 60,
    Cancel = 61,
    ToolAnswer = 62,
    ToolCancel = 63,
    Subscribe = 64,
    Unsubscribe = 65,
    ConfirmMode = 66,
    QuestionAnswer = 67,
}

impl ReqWsEventAgent {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventAgent> {
        match r#type {
            60 => Some(ReqWsEventAgent::Chat),
            61 => Some(ReqWsEventAgent::Cancel),
            62 => Some(ReqWsEventAgent::ToolAnswer),
            63 => Some(ReqWsEventAgent::ToolCancel),
            64 => Some(ReqWsEventAgent::Subscribe),
            65 => Some(ReqWsEventAgent::Unsubscribe),
            66 => Some(ReqWsEventAgent::ConfirmMode),
            67 => Some(ReqWsEventAgent::QuestionAnswer),
            _ => None,
        }
    }
}

pub enum ReqWsEventRdp {
    Open = 80,
    Close = 81,
    Input = 82,
    Resize = 83,
    Visibility = 84,
    AuthAnswer = 85,
    FrameSync = 86,
    Focus = 87,
}

pub enum ReqWsEventSerial {
    Open = 90,
    Close = 91,
    Write = 92,
}

impl ReqWsEventSerial {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventSerial> {
        match r#type {
            90 => Some(ReqWsEventSerial::Open),
            91 => Some(ReqWsEventSerial::Close),
            92 => Some(ReqWsEventSerial::Write),
            _ => None,
        }
    }
}

impl ReqWsEventRdp {
    pub fn form_number(r#type: i16) -> Option<ReqWsEventRdp> {
        match r#type {
            80 => Some(ReqWsEventRdp::Open),
            81 => Some(ReqWsEventRdp::Close),
            82 => Some(ReqWsEventRdp::Input),
            83 => Some(ReqWsEventRdp::Resize),
            84 => Some(ReqWsEventRdp::Visibility),
            85 => Some(ReqWsEventRdp::AuthAnswer),
            86 => Some(ReqWsEventRdp::FrameSync),
            87 => Some(ReqWsEventRdp::Focus),
            _ => None,
        }
    }
}
