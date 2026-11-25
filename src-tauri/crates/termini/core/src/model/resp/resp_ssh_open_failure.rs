use serde::Serialize;
use crate::model::resp::event::resp_event_ssh_auth::{OpenSshStep, RespSshOpenStepType};

pub const SSH_OPEN_FAILURE_CLASS_TCP: i8 = 0;
pub const SSH_OPEN_FAILURE_CLASS_PROBE: i8 = 1;
pub const SSH_OPEN_FAILURE_CLASS_AUTH: i8 = 2;
pub const SSH_OPEN_FAILURE_CLASS_CHANNEL: i8 = 3;
pub const SSH_OPEN_FAILURE_CLASS_CANCELLED: i8 = 4;
pub const SSH_OPEN_FAILURE_CLASS_INTERACTIVE: i8 = 5;
pub const SSH_OPEN_FAILURE_CLASS_UNKNOWN: i8 = 6;

#[derive(Debug, Clone, Serialize)]
pub struct RespSshOpenFailure {
    pub device_id: String,
    pub ok: bool,
    pub message: String,
    pub last_step: Option<i16>,
    pub failure_class: i8,
    pub hint: String,
    pub steps: Vec<i16>,
}

pub fn format_ssh_open_failure(device_id: &str, message: &str, steps: &[OpenSshStep]) -> String {
    let failure = build_ssh_open_failure(device_id, message, steps);
    serde_json::to_string(&failure).expect("RespSshOpenFailure serialize")
}

pub fn build_ssh_open_failure(device_id: &str, message: &str, steps: &[OpenSshStep]) -> RespSshOpenFailure {
    let step_codes: Vec<i16> = steps.iter().map(|step| i16::from(step.step_type)).collect();
    let last_step = step_codes.last().copied();
    let failure_class = resolve_failure_class(message, steps);
    let hint = resolve_hint(failure_class).to_string();
    RespSshOpenFailure { device_id: device_id.to_string(), ok: false, message: message.to_string(), last_step, failure_class, hint, steps: step_codes }
}

fn resolve_failure_class(message: &str, steps: &[OpenSshStep]) -> i8 {
    if message.contains("interactive prompt not allowed") {
        return SSH_OPEN_FAILURE_CLASS_INTERACTIVE;
    }
    if let Some(step) = steps.iter().rev().find(|step| is_error_step(step.step_type)) {
        return class_from_error_step(step.step_type);
    }
    if message.contains("SSH open cancelled") {
        return SSH_OPEN_FAILURE_CLASS_CANCELLED;
    }
    SSH_OPEN_FAILURE_CLASS_UNKNOWN
}

fn is_error_step(step_type: RespSshOpenStepType) -> bool {
    matches!(
        step_type,
        RespSshOpenStepType::ErrorTcpFailed
            | RespSshOpenStepType::ErrorProbeFailed
            | RespSshOpenStepType::ErrorAuthFailed
            | RespSshOpenStepType::ErrorAuthTimeout
            | RespSshOpenStepType::ErrorChannelFailed
            | RespSshOpenStepType::ErrorOpenCancelled
    )
}

fn class_from_error_step(step_type: RespSshOpenStepType) -> i8 {
    match step_type {
        RespSshOpenStepType::ErrorTcpFailed => SSH_OPEN_FAILURE_CLASS_TCP,
        RespSshOpenStepType::ErrorProbeFailed => SSH_OPEN_FAILURE_CLASS_PROBE,
        RespSshOpenStepType::ErrorAuthFailed | RespSshOpenStepType::ErrorAuthTimeout => SSH_OPEN_FAILURE_CLASS_AUTH,
        RespSshOpenStepType::ErrorChannelFailed => SSH_OPEN_FAILURE_CLASS_CHANNEL,
        RespSshOpenStepType::ErrorOpenCancelled => SSH_OPEN_FAILURE_CLASS_CANCELLED,
        RespSshOpenStepType::ProbeStart
        | RespSshOpenStepType::ProbeTcpConnecting
        | RespSshOpenStepType::ProbeTcpConnected
        | RespSshOpenStepType::ProbeNegotiating
        | RespSshOpenStepType::ProbeCompleted
        | RespSshOpenStepType::AuthStart
        | RespSshOpenStepType::AuthUsernameInput
        | RespSshOpenStepType::AuthUsernameRetry
        | RespSshOpenStepType::AuthPasswordInput
        | RespSshOpenStepType::AuthPasswordRetry
        | RespSshOpenStepType::AuthPassphraseInput
        | RespSshOpenStepType::AuthPassphraseRetry
        | RespSshOpenStepType::AuthKeyPairInput
        | RespSshOpenStepType::AuthKeyPairRetry
        | RespSshOpenStepType::AuthKeyboardInteractiveInput
        | RespSshOpenStepType::AuthKeyboardInteractiveRetry
        | RespSshOpenStepType::AuthHostFingerprintConfirm
        | RespSshOpenStepType::AuthSshAgentKeySelect
        | RespSshOpenStepType::AuthMethodSelect
        | RespSshOpenStepType::SessionOpened => SSH_OPEN_FAILURE_CLASS_UNKNOWN,
    }
}

fn resolve_hint(failure_class: i8) -> &'static str {
    if failure_class == SSH_OPEN_FAILURE_CLASS_TCP {
        return "Check the device host/port, local network and firewall reachability";
    }
    if failure_class == SSH_OPEN_FAILURE_CLASS_PROBE {
        return "TCP connected but the SSH handshake failed; check that the server SSH service is healthy and protocol-compatible";
    }
    if failure_class == SSH_OPEN_FAILURE_CLASS_AUTH {
        return "Check that the configured username, password/key/passphrase are complete and correct";
    }
    if failure_class == SSH_OPEN_FAILURE_CLASS_CHANNEL {
        return "Session authenticated but channel open failed; check server channel/subsystem limits";
    }
    if failure_class == SSH_OPEN_FAILURE_CLASS_CANCELLED {
        return "Connection was cancelled";
    }
    if failure_class == SSH_OPEN_FAILURE_CLASS_INTERACTIVE {
        return "Non-interactive agent login requires a complete ConfigSsh (with credentials); the current config would trigger interactive input";
    }
    "Check last_step and message, and verify the device SSH configuration"
}
