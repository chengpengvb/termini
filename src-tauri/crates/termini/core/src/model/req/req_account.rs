use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReqAccountCallback {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqAccountLogin {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqAccountRegister {
    pub email: String,
    pub password: String,
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqAccountEmail {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqAccountCode {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqOpenSite {
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqReportError {
    pub source: String,
    pub message: String,
    pub stack: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqFeedback {
    pub content: String,
}
