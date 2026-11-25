use md5::Md5;
use rc4::Rc4;
use hmac::Mac;
use hmac::Hmac;
use sha1::Sha1;
use rc4::KeyInit;
use rand::RngCore;
use rc4::consts::U16;
use rc4::StreamCipher;
use md5::Digest as Md5Digest;
use sha1::Digest as Sha1Digest;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

type HmacMd5 = Hmac<Md5>;

pub const SEC_EXCHANGE_PKT: u16 = 0x0001;
pub const SEC_TRANSPORT_REQ: u16 = 0x0002;
pub const SEC_TRANSPORT_RSP: u16 = 0x0004;
pub const SEC_ENCRYPT: u16 = 0x0008;
pub const SEC_INFO_PKT: u16 = 0x0040;
pub const SEC_LICENSE_PKT: u16 = 0x0080;
pub const SEC_LICENSE_ENCRYPT_SC: u16 = 0x0200;
pub const SEC_SECURE_CHECKSUM: u16 = 0x0800;
pub const SEC_AUTODETECT_REQ: u16 = 0x1000;
pub const SEC_AUTODETECT_RSP: u16 = 0x2000;
pub const SEC_HEARTBEAT: u16 = 0x4000;

pub const ENCRYPTION_LEVEL_NONE: u32 = 0x0000_0000;

pub const MASTER_SECRET_LENGTH: usize = 48;
pub const SESSION_KEY_BLOB_LENGTH: usize = 48;
pub const MAC_LENGTH: usize = 16;

const SALT_A: &[u8] = b"A";
const SALT_BB: &[u8] = b"BB";
const SALT_CCC: &[u8] = b"CCC";
const SALT_X: &[u8] = b"X";
const SALT_YY: &[u8] = b"YY";
const SALT_ZZZ: &[u8] = b"ZZZ";
const SIGNATURE_LENGTH: usize = 8;
const PAD1: [u8; 40] = [0x36; 40];
const PAD2: [u8; 48] = [0x5C; 48];

pub struct Rc4Handle {
    cipher: Rc4<U16>,
}

impl Rc4Handle {
    pub fn new(key: &[u8; 16]) -> Self {
        Self { cipher: Rc4::<U16>::new(key.into()) }
    }

    pub fn apply(&mut self, data: &mut [u8]) {
        self.cipher.apply_keystream(data);
    }
}

pub fn random_bytes(length: usize) -> Vec<u8> {
    let mut buffer = vec![0u8; length];
    rand::thread_rng().fill_bytes(&mut buffer);
    buffer
}

pub fn hmac_md5(key: &[u8], data: &[&[u8]]) -> RdpResult<[u8; 16]> {
    let mut mac = <HmacMd5 as Mac>::new_from_slice(key).map_err(|error| RdpProtocolError::authentication(format!("invalid hmac-md5 key: {}", error)))?;
    for chunk in data {
        mac.update(chunk);
    }
    let output = mac.finalize().into_bytes();
    let mut digest = [0u8; 16];
    digest.copy_from_slice(&output);
    Ok(digest)
}

pub fn md5(data: &[&[u8]]) -> [u8; 16] {
    let mut hasher = Md5::new();
    for chunk in data {
        Md5Digest::update(&mut hasher, chunk);
    }
    let output = hasher.finalize();
    let mut digest = [0u8; 16];
    digest.copy_from_slice(&output);
    digest
}

pub fn write_header(cursor: &mut WriteCursor, flags: u16) {
    cursor.write_u16_le(flags);
    cursor.write_u16_le(0);
}

pub fn read_header(cursor: &mut ReadCursor<'_>) -> RdpResult<u16> {
    let flags = cursor.read_u16_le()?;
    cursor.read_u16_le()?;
    Ok(flags)
}

pub fn master_secret(premaster_secret: &[u8], client_random: &[u8], server_random: &[u8]) -> [u8; MASTER_SECRET_LENGTH] {
    let mut output = [0u8; MASTER_SECRET_LENGTH];
    output[0..16].copy_from_slice(&salted_hash(premaster_secret, SALT_A, client_random, server_random));
    output[16..32].copy_from_slice(&salted_hash(premaster_secret, SALT_BB, client_random, server_random));
    output[32..48].copy_from_slice(&salted_hash(premaster_secret, SALT_CCC, client_random, server_random));
    output
}

pub fn session_key_blob(master_secret: &[u8], client_random: &[u8], server_random: &[u8]) -> [u8; SESSION_KEY_BLOB_LENGTH] {
    let mut output = [0u8; SESSION_KEY_BLOB_LENGTH];
    output[0..16].copy_from_slice(&salted_hash(master_secret, SALT_A, server_random, client_random));
    output[16..32].copy_from_slice(&salted_hash(master_secret, SALT_BB, server_random, client_random));
    output[32..48].copy_from_slice(&salted_hash(master_secret, SALT_CCC, server_random, client_random));
    output
}

pub fn rdp_session_key_blob(master_secret: &[u8], client_random: &[u8], server_random: &[u8]) -> [u8; SESSION_KEY_BLOB_LENGTH] {
    let mut output = [0u8; SESSION_KEY_BLOB_LENGTH];
    output[0..16].copy_from_slice(&salted_hash(master_secret, SALT_X, client_random, server_random));
    output[16..32].copy_from_slice(&salted_hash(master_secret, SALT_YY, client_random, server_random));
    output[32..48].copy_from_slice(&salted_hash(master_secret, SALT_ZZZ, client_random, server_random));
    output
}

pub fn licensing_encryption_key(session_key_blob: &[u8], client_random: &[u8], server_random: &[u8]) -> [u8; MAC_LENGTH] {
    md5_16_32_32(&session_key_blob[16..32], client_random, server_random)
}

pub fn md5_16_32_32(block: &[u8], client_random: &[u8], server_random: &[u8]) -> [u8; MAC_LENGTH] {
    md5(&[block, client_random, server_random])
}

pub fn mac_signature(sign_key: &[u8], data: &[u8]) -> [u8; SIGNATURE_LENGTH] {
    first8(mac_data(sign_key, data))
}

pub fn salted_mac_signature(sign_key: &[u8], data: &[u8], use_count: u32) -> [u8; SIGNATURE_LENGTH] {
    let length = (data.len() as u32).to_le_bytes();
    let count = use_count.to_le_bytes();
    let digest = sha1(&[sign_key, &PAD1, &length, data, &count]);
    first8(md5(&[sign_key, &PAD2, &digest]))
}

pub fn key_update_hash(update_key: &[u8], key: &[u8]) -> [u8; MAC_LENGTH] {
    let digest = sha1(&[update_key, &PAD1, key]);
    md5(&[update_key, &PAD2, &digest])
}

pub fn mac_data(mac_salt_key: &[u8], data: &[u8]) -> [u8; MAC_LENGTH] {
    let length = (data.len() as u32).to_le_bytes();
    let digest = sha1(&[mac_salt_key, &PAD1, &length, data]);
    md5(&[mac_salt_key, &PAD2, &digest])
}

fn first8(mac: [u8; MAC_LENGTH]) -> [u8; SIGNATURE_LENGTH] {
    let mut signature = [0u8; SIGNATURE_LENGTH];
    signature.copy_from_slice(&mac[..SIGNATURE_LENGTH]);
    signature
}

fn salted_hash(salt: &[u8], input: &[u8], salt1: &[u8], salt2: &[u8]) -> [u8; MAC_LENGTH] {
    let digest = sha1(&[input, salt, salt1, salt2]);
    md5(&[salt, &digest])
}

fn sha1(data: &[&[u8]]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    for chunk in data {
        Sha1Digest::update(&mut hasher, chunk);
    }
    let output = hasher.finalize();
    let mut digest = [0u8; 20];
    digest.copy_from_slice(&output);
    digest
}
