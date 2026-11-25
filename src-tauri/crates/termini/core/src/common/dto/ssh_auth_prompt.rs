use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InputPrompt {
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetryPrompt {
    pub request_id: String,
    pub last_error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FingerprintPrompt {
    pub port: u16,
    pub reason: i16,
    pub host: String,
    pub sha256: String,
    pub algorithm: String,
    pub request_id: String,
    pub previous_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InteractivePrompt {
    pub request_id: String,
    pub name: String,
    pub instruction: String,
    pub questions: Vec<InteractiveQuestion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InteractiveQuestion {
    pub text: String,
    pub echo: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodSelectPrompt {
    pub request_id: String,
    pub methods: Vec<i16>,
    pub default: Option<i16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentKeyPrompt {
    pub request_id: String,
    pub agent_type: String,
    pub keys: Vec<AgentKeyEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentKeyEntry {
    pub key_id: String,
    pub comment: String,
    pub algorithm: String,
    pub fingerprint: String,
}
