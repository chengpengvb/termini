use crate::core::error::{RdpProtocolError, RdpResult};

const RESULT_SUCCESS: i64 = 0;

pub fn build_authentication_request(assertion: &str) -> Vec<u8> {
    let mut payload = format!("{{\"rdp_assertion\":\"{}\"}}", assertion).into_bytes();
    payload.push(0);
    payload
}

pub fn parse_server_nonce(payload: &[u8]) -> RdpResult<String> {
    let value = parse_json(payload)?;
    let nonce = value.get("ts_nonce").and_then(serde_json::Value::as_str).ok_or_else(|| RdpProtocolError::protocol("aad message is missing ts_nonce".to_string()))?;
    Ok(nonce.to_string())
}

pub fn parse_authentication_result(payload: &[u8]) -> RdpResult<()> {
    let value = parse_json(payload)?;
    let result = value.get("authentication_result").and_then(serde_json::Value::as_i64).ok_or_else(|| RdpProtocolError::protocol("aad message is missing authentication_result".to_string()))?;
    if result != RESULT_SUCCESS {
        return Err(RdpProtocolError::rejected(format!("Azure AD authentication failed: authentication_result={}", result)));
    }
    Ok(())
}

fn parse_json(payload: &[u8]) -> RdpResult<serde_json::Value> {
    let end = payload.iter().position(|byte| *byte == 0).unwrap_or(payload.len());
    let text = std::str::from_utf8(&payload[..end]).map_err(|error| RdpProtocolError::encoding(format!("aad message is not UTF-8: {}", error)))?;
    serde_json::from_str(text).map_err(|error| RdpProtocolError::protocol(format!("failed to parse aad message: {}", error)))
}
