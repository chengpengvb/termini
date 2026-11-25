use base64::Engine;
use rsa::Pkcs1v15Sign;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use rsa::traits::PublicKeyParts;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::module::rdp::protocol::core::aad;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use crate::module::rdp::client_rdp_session::RdpSettings;
use crate::module::rdp::protocol::core::transport::RdpTransport;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};


const KEY_BITS: usize = 2048;
const REQUEST_LIMIT: usize = 8192;
const VERIFIER_LENGTH: usize = 32;
pub(crate) const DEFAULT_TENANT: &str = "common";
const AUTHORITY_HOST: &str = "login.microsoftonline.com";
const AUDIENCE_PREFIX: &str = "ms-device-service://termsrv.wvd.microsoft.com/name/";
const SCOPE_TEMPLATE: &str = "ms-device-service://termsrv.wvd.microsoft.com/name/{host}/user_impersonation";
const REDIRECT_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<html><body>Azure sign-in complete. You can close this page.</body></html>";
const SHA256_DIGEST_INFO_PREFIX: [u8; 19] = [0x30, 0x31, 0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 0x00, 0x04, 0x20];

struct AadSettings {
    client_id: String,
    tenant: String,
    hostname: String,
}

struct AadIdentity {
    key: RsaPrivateKey,
    kid: String,
    exponent: String,
    modulus: String,
    access_token: String,
    nonce: String,
    hostname: String,
}

struct Endpoints {
    authorization_endpoint: String,
    token_endpoint: String,
}

impl RdpSessionFactory {
    pub(crate) async fn authenticate_aad(&self, transport: &mut RdpTransport, settings: &RdpSettings) -> RdpResult<()> {
        let client_id = settings.aad_client_id.clone().ok_or_else(|| RdpProtocolError::authentication("server selected Azure AD authentication but no client ID is configured".to_string()))?;
        let aad_settings = AadSettings { client_id, hostname: settings.host.clone(), tenant: settings.aad_tenant.clone().unwrap_or_else(|| DEFAULT_TENANT.to_string()) };
        let identity = acquire_identity(&aad_settings).await?;
        let server_nonce = aad::parse_server_nonce(&transport.read_terminated().await?)?;
        let assertion = build_assertion(&identity, &server_nonce)?;
        transport.send(&aad::build_authentication_request(&assertion)).await?;
        aad::parse_authentication_result(&transport.read_terminated().await?)
    }
}

async fn acquire_identity(settings: &AadSettings) -> RdpResult<AadIdentity> {
    let key = RsaPrivateKey::new(&mut rand::thread_rng(), KEY_BITS).map_err(|error| RdpProtocolError::protocol(format!("failed to generate PoP key: {}", error)))?;
    let exponent = URL_SAFE_NO_PAD.encode(key.e().to_bytes_be());
    let modulus = URL_SAFE_NO_PAD.encode(key.n().to_bytes_be());
    let kid = build_kid(&exponent, &modulus);

    let endpoints = fetch_endpoints(settings).await?;
    let access_token = acquire_token(settings, &endpoints, &kid).await?;
    let nonce = fetch_nonce(&endpoints.token_endpoint).await?;
    Ok(AadIdentity { key, kid, exponent, modulus, access_token, nonce, hostname: short_hostname(&settings.hostname) })
}

fn build_assertion(identity: &AadIdentity, server_nonce: &str) -> RdpResult<String> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| RdpProtocolError::protocol(format!("failed to read system time: {}", error)))?.as_secs();
    let header = URL_SAFE_NO_PAD.encode(format!("{{\"alg\":\"RS256\",\"kid\":\"{}\"}}", identity.kid));
    let payload = URL_SAFE_NO_PAD.encode(format!(
        "{{\"ts\":\"{}\",\"at\":\"{}\",\"u\":\"{}{}\",\"nonce\":\"{}\",\"cnf\":{{\"jwk\":{{\"kty\":\"RSA\",\"e\":\"{}\",\"n\":\"{}\"}}}},\"client_claims\":\"{{\\\"aad_nonce\\\":\\\"{}\\\"}}\"}}",
        timestamp, identity.access_token, AUDIENCE_PREFIX, identity.hostname, server_nonce, identity.exponent, identity.modulus, identity.nonce
    ));
    let mut digest_info = SHA256_DIGEST_INFO_PREFIX.to_vec();
    digest_info.extend_from_slice(&Sha256::digest(format!("{}.{}", header, payload).as_bytes()));
    let signature = identity.key.sign(Pkcs1v15Sign::new_unprefixed(), &digest_info).map_err(|error| RdpProtocolError::protocol(format!("failed to sign rdp_assertion: {}", error)))?;
    Ok(format!("{}.{}.{}", header, payload, URL_SAFE_NO_PAD.encode(signature)))
}

fn build_kid(exponent: &str, modulus: &str) -> String {
    let jwk = format!("{{\"e\":\"{}\",\"kty\":\"RSA\",\"n\":\"{}\"}}", exponent, modulus);
    let thumbprint = URL_SAFE_NO_PAD.encode(Sha256::digest(jwk.as_bytes()));
    URL_SAFE_NO_PAD.encode(format!("{{\"kid\":\"{}\"}}", thumbprint))
}

fn short_hostname(hostname: &str) -> String {
    hostname.split('.').next().unwrap_or(hostname).to_string()
}

async fn fetch_endpoints(settings: &AadSettings) -> RdpResult<Endpoints> {
    let url = format!("https://{}/{}/v2.0/.well-known/openid-configuration", AUTHORITY_HOST, settings.tenant);
    let response = reqwest::get(&url).await.map_err(|error| RdpProtocolError::protocol(format!("failed to fetch Azure configuration: {}", error)))?;
    let document: serde_json::Value = response.json().await.map_err(|error| RdpProtocolError::protocol(format!("failed to parse Azure configuration: {}", error)))?;
    let authorization_endpoint = read_string(&document, "authorization_endpoint")?;
    let token_endpoint = read_string(&document, "token_endpoint")?;
    Ok(Endpoints { authorization_endpoint, token_endpoint })
}

async fn acquire_token(settings: &AadSettings, endpoints: &Endpoints, kid: &str) -> RdpResult<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|error| RdpProtocolError::protocol(format!("failed to start local callback listener: {}", error)))?;
    let port = listener.local_addr().map_err(|error| RdpProtocolError::protocol(format!("failed to read callback port: {}", error)))?.port();
    let redirect_uri = format!("http://localhost:{}", port);

    let verifier = URL_SAFE_NO_PAD.encode(crate::module::rdp::protocol::core::security::random_bytes(VERIFIER_LENGTH));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = URL_SAFE_NO_PAD.encode(crate::module::rdp::protocol::core::security::random_bytes(VERIFIER_LENGTH));
    let scope = SCOPE_TEMPLATE.replace("{host}", &short_hostname(&settings.hostname));

    let authorize = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&response_mode=query&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        endpoints.authorization_endpoint,
        urlencoding(&settings.client_id),
        urlencoding(&redirect_uri),
        urlencoding(&scope),
        state,
        challenge
    );
    tauri_plugin_opener::open_url(&authorize, None::<&str>).map_err(|error| RdpProtocolError::protocol(format!("failed to open browser: {}", error)))?;

    let code = wait_for_code(&listener, &state).await?;
    exchange_code(endpoints, settings, &redirect_uri, &verifier, &code, &scope, kid).await
}

async fn wait_for_code(listener: &TcpListener, state: &str) -> RdpResult<String> {
    let (mut stream, _) = listener.accept().await.map_err(|error| RdpProtocolError::protocol(format!("failed to accept Azure callback: {}", error)))?;
    let mut buffer = vec![0u8; REQUEST_LIMIT];
    let read = stream.read(&mut buffer).await.map_err(|error| RdpProtocolError::protocol(format!("failed to read Azure callback: {}", error)))?;
    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
    let _ = stream.write_all(REDIRECT_RESPONSE.as_bytes()).await;
    let _ = stream.shutdown().await;

    let query = request.split_whitespace().nth(1).and_then(|target| target.split_once('?')).map(|(_, query)| query.to_string()).unwrap_or_default();
    let mut code = None;
    let mut returned_state = None;
    let mut error = None;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "code" => code = Some(value.to_string()),
            "state" => returned_state = Some(value.to_string()),
            "error_description" => error = Some(value.replace('+', " ")),
            "error" if error.is_none() => error = Some(value.to_string()),
            _ => {}
        }
    }
    if let Some(error) = error {
        return Err(RdpProtocolError::authentication(format!("Azure sign-in was rejected: {}", error)));
    }
    if returned_state.as_deref() != Some(state) {
        return Err(RdpProtocolError::authentication("Azure callback state mismatch".to_string()));
    }
    code.ok_or_else(|| RdpProtocolError::authentication("Azure callback is missing the authorization code".to_string()))
}

async fn exchange_code(endpoints: &Endpoints, settings: &AadSettings, redirect_uri: &str, verifier: &str, code: &str, scope: &str, kid: &str) -> RdpResult<String> {
    let request_cnf = URL_SAFE_NO_PAD.encode(format!("{{\"kid\":\"{}\"}}", kid));
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", settings.client_id.as_str()),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier),
        ("scope", scope),
        ("token_type", "pop"),
        ("req_cnf", request_cnf.as_str()),
    ];
    let response = reqwest::Client::new().post(&endpoints.token_endpoint).form(&form).send().await.map_err(|error| RdpProtocolError::protocol(format!("failed to request Azure token: {}", error)))?;
    let document: serde_json::Value = response.json().await.map_err(|error| RdpProtocolError::protocol(format!("failed to parse Azure token: {}", error)))?;
    if let Some(error) = document.get("error_description").and_then(serde_json::Value::as_str) {
        return Err(RdpProtocolError::authentication(format!("Azure refused to issue a token: {}", error)));
    }
    read_string(&document, "access_token")
}

async fn fetch_nonce(token_endpoint: &str) -> RdpResult<String> {
    let response = reqwest::Client::new()
        .post(token_endpoint)
        .form(&[("grant_type", "srv_challenge")])
        .send()
        .await
        .map_err(|error| RdpProtocolError::protocol(format!("failed to request Azure nonce: {}", error)))?;
    let document: serde_json::Value = response.json().await.map_err(|error| RdpProtocolError::protocol(format!("failed to parse Azure nonce: {}", error)))?;
    read_string(&document, "Nonce")
}

fn read_string(document: &serde_json::Value, field: &str) -> RdpResult<String> {
    document.get(field).and_then(serde_json::Value::as_str).map(|value| value.to_string()).ok_or_else(|| RdpProtocolError::protocol(format!("Azure response is missing field {}", field)))
}

fn urlencoding(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (byte as char).to_string(),
            _ => format!("%{:02X}", byte),
        })
        .collect()
}
