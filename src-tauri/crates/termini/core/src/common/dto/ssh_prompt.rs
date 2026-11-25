use crate::model::resp::event::resp_event_ssh_auth::{RespSshOpenStepType, SshOpenCallback};

pub struct SshPrompt<'a> {
    pub channel_id: &'a str,
    pub request_id: &'a str,
    pub payload: serde_json::Value,
    pub callback: &'a SshOpenCallback,
    pub step_type: RespSshOpenStepType,
}

impl<'a> SshPrompt<'a> {
    pub fn new(channel_id: &'a str, request_id: &'a str, step_type: RespSshOpenStepType, callback: &'a SshOpenCallback, payload: serde_json::Value) -> Self {
        Self { channel_id, request_id, step_type, callback, payload }
    }
}
