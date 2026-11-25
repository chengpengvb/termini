use crate::crypto::rsa;
use crate::core::security;
use crate::crypto::certificate;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

const LICENSE_REQUEST: u8 = 0x01;
const PLATFORM_CHALLENGE: u8 = 0x02;
const NEW_LICENSE: u8 = 0x03;
const UPGRADE_LICENSE: u8 = 0x04;
const LICENSE_INFO: u8 = 0x12;
const NEW_LICENSE_REQUEST: u8 = 0x13;
const PLATFORM_CHALLENGE_RESPONSE: u8 = 0x15;
const ERROR_ALERT: u8 = 0xFF;

const PREAMBLE_LENGTH: usize = 4;
const PREAMBLE_VERSION_3_0: u8 = 0x03;
const EXTENDED_ERROR_MSG_SUPPORTED: u8 = 0x80;

const BB_DATA_BLOB: u16 = 0x0001;
const BB_RANDOM_BLOB: u16 = 0x0002;
const BB_ENCRYPTED_DATA_BLOB: u16 = 0x0009;
const BB_CLIENT_USER_NAME_BLOB: u16 = 0x000F;
const BB_CLIENT_MACHINE_NAME_BLOB: u16 = 0x0010;

const KEY_EXCHANGE_ALG_RSA: u32 = 0x0000_0001;
const PLATFORM_ID: u32 = 0x0401_0000;
const OTHER_PLATFORM_CHALLENGE_TYPE: u16 = 0xFF00;
const LICENSE_DETAIL_DETAIL: u16 = 0x0003;
const PLATFORM_CHALLENGE_RESPONSE_VERSION: u16 = 0x0100;

const RANDOM_LENGTH: usize = 32;
const PREMASTER_SECRET_LENGTH: usize = 48;
const PREMASTER_PADDING_LENGTH: usize = 8;
const HWID_PLATFORM_ID_LENGTH: usize = 4;
const HWID_LENGTH: usize = 20;

const STATUS_VALID_CLIENT: u32 = 0x0000_0007;

const ERR_INVALID_SERVER_CERTIFICATE: u32 = 0x0000_0001;
const ERR_NO_LICENSE: u32 = 0x0000_0002;
const ERR_INVALID_MAC: u32 = 0x0000_0003;
const ERR_INVALID_SCOPE: u32 = 0x0000_0004;
const ERR_NO_LICENSE_SERVER: u32 = 0x0000_0006;
const ERR_INVALID_CLIENT: u32 = 0x0000_0008;
const ERR_INVALID_PRODUCT_ID: u32 = 0x0000_000B;
const ERR_INVALID_MESSAGE_LEN: u32 = 0x0000_000C;

pub enum LicenseOutcome {
    Complete(Option<Vec<u8>>),
    Respond(Vec<u8>),
}

pub struct LicenseState {
    username: String,
    machine_name: String,
    license: Option<Vec<u8>>,
    keys: Option<LicenseKeys>,
}

struct LicenseKeys {
    mac_salt_key: [u8; security::MAC_LENGTH],
    licensing_key: [u8; security::MAC_LENGTH],
    hardware_id: [u8; HWID_LENGTH],
}

impl LicenseState {
    pub fn new(username: String, machine_name: String, license: Option<Vec<u8>>) -> Self {
        Self { username, machine_name, license, keys: None }
    }

    fn keys(&self) -> RdpResult<&LicenseKeys> {
        self.keys.as_ref().ok_or_else(|| RdpProtocolError::protocol("licensing keys missing: server did not send a license request first".to_string()))
    }
}

pub fn process(state: &mut LicenseState, payload: &[u8]) -> RdpResult<LicenseOutcome> {
    let mut cursor = ReadCursor::new(payload);
    let flags = security::read_header(&mut cursor)?;
    if flags & security::SEC_LICENSE_PKT == 0 {
        return Err(RdpProtocolError::protocol(format!("received non-licensing PDU during licensing phase: flags=0x{:04x}", flags)));
    }
    let message_type = cursor.read_u8()?;
    cursor.read_u8()?;
    let size = usize::from(cursor.read_u16_le()?);
    let body = size.checked_sub(PREAMBLE_LENGTH).ok_or_else(|| RdpProtocolError::protocol(format!("invalid licensing message length: {}", size)))?;
    cursor.ensure(body)?;

    match message_type {
        LICENSE_REQUEST => answer_license_request(state, &mut cursor),
        PLATFORM_CHALLENGE => answer_platform_challenge(state, &mut cursor),
        NEW_LICENSE | UPGRADE_LICENSE => accept_license(state, &mut cursor),
        ERROR_ALERT => read_error_alert(&mut cursor),
        other => Err(RdpProtocolError::protocol(format!("unknown licensing message type: 0x{:02x}", other))),
    }
}

fn answer_license_request(state: &mut LicenseState, cursor: &mut ReadCursor<'_>) -> RdpResult<LicenseOutcome> {
    let server_random = cursor.read_slice(RANDOM_LENGTH)?;
    read_product_info(cursor)?;
    read_blob(cursor)?;
    let server_certificate = read_blob(cursor)?;
    read_scope_list(cursor)?;

    let public_key = certificate::parse_server_certificate(server_certificate)?;
    let client_random = security::random_bytes(RANDOM_LENGTH);
    let premaster_secret = security::random_bytes(PREMASTER_SECRET_LENGTH);
    let master_secret = security::master_secret(&premaster_secret, &client_random, server_random);
    let session_key_blob = security::session_key_blob(&master_secret, &client_random, server_random);

    let mut mac_salt_key = [0u8; security::MAC_LENGTH];
    mac_salt_key.copy_from_slice(&session_key_blob[..security::MAC_LENGTH]);
    let licensing_key = security::licensing_encryption_key(&session_key_blob, &client_random, server_random);
    let hardware_id = build_hardware_id(&state.machine_name);
    let encrypted_premaster_secret = rsa::public_encrypt(&premaster_secret, &public_key.modulus, &public_key.exponent)?;
    let keys = LicenseKeys { mac_salt_key, licensing_key, hardware_id };

    let mut body = WriteCursor::with_capacity(encrypted_premaster_secret.len() + 128);
    body.write_u32_le(KEY_EXCHANGE_ALG_RSA);
    body.write_u32_le(PLATFORM_ID);
    body.write_slice(&client_random);
    write_premaster_blob(&mut body, &encrypted_premaster_secret)?;
    let message_type = match &state.license {
        Some(license) => {
            write_blob(&mut body, BB_DATA_BLOB, license)?;
            write_blob(&mut body, BB_ENCRYPTED_DATA_BLOB, &encrypt(&keys, &keys.hardware_id))?;
            body.write_slice(&security::mac_data(&keys.mac_salt_key, &keys.hardware_id));
            LICENSE_INFO
        }
        None => {
            write_blob(&mut body, BB_CLIENT_USER_NAME_BLOB, &terminated(&state.username))?;
            write_blob(&mut body, BB_CLIENT_MACHINE_NAME_BLOB, &terminated(&state.machine_name))?;
            NEW_LICENSE_REQUEST
        }
    };
    state.keys = Some(keys);
    Ok(LicenseOutcome::Respond(build_pdu(message_type, body.as_slice())?))
}

fn answer_platform_challenge(state: &LicenseState, cursor: &mut ReadCursor<'_>) -> RdpResult<LicenseOutcome> {
    let keys = state.keys()?;
    cursor.read_u32_le()?;
    let encrypted_challenge = read_blob(cursor)?;
    let signature = cursor.read_slice(security::MAC_LENGTH)?;
    let challenge = decrypt_verified(keys, encrypted_challenge, signature)?;

    let mut response = WriteCursor::with_capacity(challenge.len() + 8);
    response.write_u16_le(PLATFORM_CHALLENGE_RESPONSE_VERSION);
    response.write_u16_le(OTHER_PLATFORM_CHALLENGE_TYPE);
    response.write_u16_le(LICENSE_DETAIL_DETAIL);
    response.write_u16_le(blob_length(challenge.len())?);
    response.write_slice(&challenge);
    let response = response.into_vec();

    let mut signed = response.clone();
    signed.extend_from_slice(&keys.hardware_id);
    let mac_data = security::mac_data(&keys.mac_salt_key, &signed);

    let mut body = WriteCursor::with_capacity(response.len() + 64);
    write_blob(&mut body, BB_ENCRYPTED_DATA_BLOB, &encrypt(keys, &response))?;
    write_blob(&mut body, BB_ENCRYPTED_DATA_BLOB, &encrypt(keys, &keys.hardware_id))?;
    body.write_slice(&mac_data);
    Ok(LicenseOutcome::Respond(build_pdu(PLATFORM_CHALLENGE_RESPONSE, body.as_slice())?))
}

fn accept_license(state: &LicenseState, cursor: &mut ReadCursor<'_>) -> RdpResult<LicenseOutcome> {
    let keys = state.keys()?;
    let encrypted_license = read_blob(cursor)?;
    let signature = cursor.read_slice(security::MAC_LENGTH)?;
    let license = decrypt_verified(keys, encrypted_license, signature)?;
    Ok(LicenseOutcome::Complete(Some(read_license_info(&license)?)))
}

fn read_license_info(license: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = ReadCursor::new(license);
    cursor.skip(4)?;
    let scope = cursor.read_u32_le()? as usize;
    cursor.skip(scope)?;
    let company_name = cursor.read_u32_le()? as usize;
    cursor.skip(company_name)?;
    let product_id = cursor.read_u32_le()? as usize;
    cursor.skip(product_id)?;
    let length = cursor.read_u32_le()? as usize;
    cursor.read_vec(length)
}

fn read_error_alert(cursor: &mut ReadCursor<'_>) -> RdpResult<LicenseOutcome> {
    let error_code = cursor.read_u32_le()?;
    if error_code == STATUS_VALID_CLIENT {
        return Ok(LicenseOutcome::Complete(None));
    }
    let state_transition = cursor.read_u32_le()?;
    Err(RdpProtocolError::rejected(format!("RDS licensing failed: {} (transition={})", describe_license_error(error_code), state_transition)))
}

fn build_hardware_id(machine_name: &str) -> [u8; HWID_LENGTH] {
    let mut hardware_id = [0u8; HWID_LENGTH];
    hardware_id[..HWID_PLATFORM_ID_LENGTH].copy_from_slice(&PLATFORM_ID.to_le_bytes());
    hardware_id[HWID_PLATFORM_ID_LENGTH..].copy_from_slice(&security::md5(&[machine_name.as_bytes()]));
    hardware_id
}

fn encrypt(keys: &LicenseKeys, input: &[u8]) -> Vec<u8> {
    let mut output = input.to_vec();
    security::Rc4Handle::new(&keys.licensing_key).apply(&mut output);
    output
}

fn decrypt_verified(keys: &LicenseKeys, input: &[u8], signature: &[u8]) -> RdpResult<Vec<u8>> {
    let output = encrypt(keys, input);
    if security::mac_data(&keys.mac_salt_key, &output).as_slice() != signature {
        return Err(RdpProtocolError::authentication("licensing data MAC verification failed".to_string()));
    }
    Ok(output)
}

fn build_pdu(message_type: u8, body: &[u8]) -> RdpResult<Vec<u8>> {
    let size = u16::try_from(body.len() + PREAMBLE_LENGTH).map_err(|_| RdpProtocolError::encoding(format!("licensing message too long: {}", body.len())))?;
    let mut cursor = WriteCursor::with_capacity(body.len() + 8);
    cursor.write_u8(message_type);
    cursor.write_u8(PREAMBLE_VERSION_3_0 | EXTENDED_ERROR_MSG_SUPPORTED);
    cursor.write_u16_le(size);
    cursor.write_slice(body);
    Ok(cursor.into_vec())
}

fn read_product_info(cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    cursor.read_u32_le()?;
    let company_name = cursor.read_u32_le()? as usize;
    if company_name < 2 || company_name % 2 != 0 {
        return Err(RdpProtocolError::protocol(format!("invalid licensing product info company name length: {}", company_name)));
    }
    cursor.skip(company_name)?;
    let product_id = cursor.read_u32_le()? as usize;
    if product_id < 2 || product_id % 2 != 0 {
        return Err(RdpProtocolError::protocol(format!("invalid licensing product info product id length: {}", product_id)));
    }
    cursor.skip(product_id)
}

fn read_scope_list(cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    let count = cursor.read_u32_le()?;
    for _ in 0..count {
        read_blob(cursor)?;
    }
    Ok(())
}

fn read_blob<'a>(cursor: &mut ReadCursor<'a>) -> RdpResult<&'a [u8]> {
    cursor.read_u16_le()?;
    let length = usize::from(cursor.read_u16_le()?);
    cursor.read_slice(length)
}

fn write_blob(cursor: &mut WriteCursor, blob_type: u16, data: &[u8]) -> RdpResult<()> {
    cursor.write_u16_le(blob_type);
    cursor.write_u16_le(blob_length(data.len())?);
    cursor.write_slice(data);
    Ok(())
}

fn write_premaster_blob(cursor: &mut WriteCursor, data: &[u8]) -> RdpResult<()> {
    cursor.write_u16_le(BB_RANDOM_BLOB);
    cursor.write_u16_le(blob_length(data.len() + PREMASTER_PADDING_LENGTH)?);
    cursor.write_slice(data);
    cursor.write_zeros(PREMASTER_PADDING_LENGTH);
    Ok(())
}

fn blob_length(length: usize) -> RdpResult<u16> {
    u16::try_from(length).map_err(|_| RdpProtocolError::encoding(format!("licensing blob too long: {}", length)))
}

fn terminated(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    bytes
}

fn describe_license_error(code: u32) -> String {
    let reason = match code {
        ERR_INVALID_SERVER_CERTIFICATE => "invalid server certificate",
        ERR_NO_LICENSE => "no license available",
        ERR_INVALID_MAC => "invalid licensing MAC",
        ERR_INVALID_SCOPE => "invalid license scope",
        ERR_NO_LICENSE_SERVER => "no license server found",
        ERR_INVALID_CLIENT => "invalid client",
        ERR_INVALID_PRODUCT_ID => "invalid product id",
        ERR_INVALID_MESSAGE_LEN => "invalid licensing message length",
        _ => "unknown licensing error",
    };
    format!("{} (0x{:08x})", reason, code)
}
