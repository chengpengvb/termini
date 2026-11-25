use sspi::ntlm::NtlmConfig;
use crate::core::error::{RdpProtocolError, RdpResult};
use sspi::credssp::{ClientMode, ClientState, CredSspClient, CredSspMode};

pub const AUTHZ_SUCCESS: u32 = 0x0000_0000;
const AUTHZ_ACCESS_DENIED: u32 = 0x0000_0005;

pub fn new_client(host: &str, domain: &str, user: &str, password: &str, workstation: &str, public_key: Vec<u8>) -> RdpResult<CredSspClient> {
    let username = sspi::Username::new(user, Some(domain)).map_err(|error| RdpProtocolError::authentication(error.to_string()))?;
    let credentials = sspi::Credentials::AuthIdentity(sspi::AuthIdentity { username, password: password.to_string().into() });
    let client_mode = if domain.is_empty() {
        ClientMode::Ntlm(NtlmConfig::new(workstation.to_string()))
    } else {
        ClientMode::Negotiate(sspi::NegotiateConfig::new(
            Box::new(sspi::KerberosConfig { kdc_url: sspi::detect_kdc_url(domain), client_computer_name: workstation.to_string() }),
            Some("kerberos,!ntlm".to_string()),
            workstation.to_string(),
        ))
    };
    CredSspClient::new(public_key, credentials, CredSspMode::WithCredentials, client_mode, format!("TERMSRV/{}", host)).map_err(map_sspi)
}

pub fn process(client: &mut CredSspClient, incoming: sspi::credssp::TsRequest) -> RdpResult<ClientState> {
    client.process(incoming).resolve_with_default_network_client().map_err(map_sspi)
}

pub fn encode_sspi_ts_request(request: &sspi::credssp::TsRequest) -> RdpResult<Vec<u8>> {
    let mut buffer = Vec::new();
    request.encode_ts_request(&mut buffer).map_err(|error| RdpProtocolError::encoding(error.to_string()))?;
    Ok(buffer)
}

pub fn decode_sspi_ts_request(data: &[u8]) -> RdpResult<sspi::credssp::TsRequest> {
    sspi::credssp::TsRequest::from_buffer(data).map_err(|error| RdpProtocolError::encoding(error.to_string()))
}

fn map_sspi(error: sspi::Error) -> RdpProtocolError {
    RdpProtocolError::authentication(error.to_string())
}

pub fn describe_early_user_auth(result: u32) -> String {
    let reason = match result {
        AUTHZ_ACCESS_DENIED => "the server denied this user access to the session",
        _ => "server early user authorization failed",
    };
    format!("{} (authzResult=0x{:08x})", reason, result)
}
