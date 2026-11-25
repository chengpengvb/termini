use rc4::Rc4;
use rc4::KeyInit;
use rc4::StreamCipher;
use crate::core::security;
use rc4::consts::{U16, U8};
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const ENCRYPTION_METHOD_40BIT: u32 = 0x0000_0001;
pub const ENCRYPTION_METHOD_128BIT: u32 = 0x0000_0002;
pub const ENCRYPTION_METHOD_56BIT: u32 = 0x0000_0008;
pub const ENCRYPTION_METHOD_FIPS: u32 = 0x0000_0010;

pub const FASTPATH_ENCRYPTED: u8 = 0x02;
pub const FASTPATH_SECURE_CHECKSUM: u8 = 0x01;

const KEY_LENGTH: usize = 16;
const SHORT_KEY_LENGTH: usize = 8;
const SIGNATURE_LENGTH: usize = 8;
const KEY_UPDATE_THRESHOLD: u32 = 4096;
const SALT: [u8; 3] = [0xD1, 0x26, 0x9E];

enum Rc4Stream {
    Short(Rc4<U8>),
    Long(Rc4<U16>),
}

impl Rc4Stream {
    fn new(key: &[u8]) -> RdpResult<Self> {
        match key.len() {
            SHORT_KEY_LENGTH => Ok(Rc4Stream::Short(Rc4::<U8>::new_from_slice(key).map_err(|error| RdpProtocolError::protocol(format!("invalid RC4 key: {}", error)))?)),
            KEY_LENGTH => Ok(Rc4Stream::Long(Rc4::<U16>::new_from_slice(key).map_err(|error| RdpProtocolError::protocol(format!("invalid RC4 key: {}", error)))?)),
            other => Err(RdpProtocolError::protocol(format!("invalid RC4 key length: {}", other))),
        }
    }

    fn apply(&mut self, data: &mut [u8]) {
        match self {
            Rc4Stream::Short(cipher) => cipher.apply_keystream(data),
            Rc4Stream::Long(cipher) => cipher.apply_keystream(data),
        }
    }
}

pub struct SecurityContext {
    key_length: usize,
    salt_length: usize,
    sign_key: [u8; KEY_LENGTH],
    encrypt_key: [u8; KEY_LENGTH],
    decrypt_key: [u8; KEY_LENGTH],
    encrypt_update_key: [u8; KEY_LENGTH],
    decrypt_update_key: [u8; KEY_LENGTH],
    encrypt: Rc4Stream,
    decrypt: Rc4Stream,
    encrypt_use_count: u32,
    decrypt_use_count: u32,
    encrypt_checksum_use_count: u32,
    decrypt_checksum_use_count: u32,
    salted_checksum: bool,
}

#[derive(Default)]
pub enum RdpSecurity {
    #[default]
    None,
    Standard(Box<SecurityContext>),
}

impl RdpSecurity {
    pub fn establish(method: u32, client_random: &[u8], server_random: &[u8]) -> RdpResult<Self> {
        Self::from_role(method, client_random, server_random, false)
    }

    pub fn establish_server(method: u32, client_random: &[u8], server_random: &[u8]) -> RdpResult<Self> {
        Self::from_role(method, client_random, server_random, true)
    }

    fn from_role(method: u32, client_random: &[u8], server_random: &[u8], server: bool) -> RdpResult<Self> {
        if method == ENCRYPTION_METHOD_FIPS {
            return Err(RdpProtocolError::unsupported("server requires FIPS encryption (3DES); only RC4 is implemented".to_string()));
        }
        Ok(RdpSecurity::Standard(Box::new(SecurityContext::establish(method, client_random, server_random, server)?)))
    }

    pub fn wrap(&mut self, flags: u16, body: &[u8]) -> RdpResult<Vec<u8>> {
        match self {
            RdpSecurity::None => {
                if flags == 0 {
                    return Ok(body.to_vec());
                }
                let mut cursor = WriteCursor::with_capacity(body.len() + 4);
                security::write_header(&mut cursor, flags);
                cursor.write_slice(body);
                Ok(cursor.into_vec())
            }
            RdpSecurity::Standard(context) => {
                let mut payload = body.to_vec();
                let salted = context.salted_checksum;
                let signature = context.sign(&payload, true, salted)?;
                context.encrypt(&mut payload)?;
                let mut wrap_flags = flags | security::SEC_ENCRYPT;
                if salted {
                    wrap_flags |= security::SEC_SECURE_CHECKSUM;
                }
                let mut cursor = WriteCursor::with_capacity(payload.len() + 12);
                security::write_header(&mut cursor, wrap_flags);
                cursor.write_slice(&signature);
                cursor.write_slice(&payload);
                Ok(cursor.into_vec())
            }
        }
    }

    pub fn unwrap(&mut self, payload: &[u8]) -> RdpResult<Vec<u8>> {
        let RdpSecurity::Standard(context) = self else {
            return Ok(payload.to_vec());
        };
        let mut cursor = ReadCursor::new(payload);
        let flags = security::read_header(&mut cursor)?;
        if flags & security::SEC_ENCRYPT == 0 {
            return Ok(rebuild(flags, cursor.rest()));
        }
        let signature = cursor.read_vec(SIGNATURE_LENGTH)?;
        let mut body = cursor.rest().to_vec();
        context.decrypt(&mut body)?;
        let expected = context.sign(&body, false, flags & security::SEC_SECURE_CHECKSUM != 0)?;
        if expected != signature.as_slice() {
            return Err(RdpProtocolError::authentication("RDP security layer signature verification failed".to_string()));
        }
        Ok(rebuild(flags, &body))
    }

    pub fn wrap_fastpath(&mut self, events: &[u8]) -> RdpResult<(u8, Vec<u8>)> {
        let RdpSecurity::Standard(context) = self else {
            return Ok((0, events.to_vec()));
        };
        let salted = context.salted_checksum;
        let mut payload = events.to_vec();
        let signature = context.sign(&payload, true, salted)?;
        context.encrypt(&mut payload)?;
        let mut body = Vec::with_capacity(payload.len() + SIGNATURE_LENGTH);
        body.extend_from_slice(&signature);
        body.extend_from_slice(&payload);
        let mut flags = FASTPATH_ENCRYPTED;
        if salted {
            flags |= FASTPATH_SECURE_CHECKSUM;
        }
        Ok((flags, body))
    }

    pub fn unwrap_fastpath(&mut self, flags: u8, payload: &[u8]) -> RdpResult<Vec<u8>> {
        if flags & FASTPATH_ENCRYPTED == 0 {
            return Ok(payload.to_vec());
        }
        let RdpSecurity::Standard(context) = self else {
            return Err(RdpProtocolError::protocol(format!("fastpath frame has RDP-layer encryption enabled but no keys are established: flags={}", flags)));
        };
        let mut cursor = ReadCursor::new(payload);
        let signature = cursor.read_vec(SIGNATURE_LENGTH)?;
        let mut body = cursor.rest().to_vec();
        context.decrypt(&mut body)?;
        let expected = context.sign(&body, false, flags & FASTPATH_SECURE_CHECKSUM != 0)?;
        if expected != signature.as_slice() {
            return Err(RdpProtocolError::authentication("fastpath signature verification failed".to_string()));
        }
        Ok(body)
    }

    pub fn set_salted_checksum(&mut self, salted: bool) {
        if let RdpSecurity::Standard(context) = self {
            context.salted_checksum = salted;
        }
    }
}

fn rebuild(flags: u16, body: &[u8]) -> Vec<u8> {
    let typed = flags & !(security::SEC_ENCRYPT | security::SEC_SECURE_CHECKSUM);
    if typed == 0 {
        return body.to_vec();
    }
    let mut cursor = WriteCursor::with_capacity(body.len() + 4);
    security::write_header(&mut cursor, typed);
    cursor.write_slice(body);
    cursor.into_vec()
}

impl SecurityContext {
    fn establish(method: u32, client_random: &[u8], server_random: &[u8], server: bool) -> RdpResult<Self> {
        if client_random.len() < 24 || server_random.len() < 24 {
            return Err(RdpProtocolError::protocol(format!("security layer random too short: client {} server {}", client_random.len(), server_random.len())));
        }
        let mut premaster = Vec::with_capacity(48);
        premaster.extend_from_slice(&client_random[..24]);
        premaster.extend_from_slice(&server_random[..24]);
        let master = security::master_secret(&premaster, client_random, server_random);
        let session = security::rdp_session_key_blob(&master, client_random, server_random);

        let mut sign_key = [0u8; KEY_LENGTH];
        sign_key.copy_from_slice(&session[..KEY_LENGTH]);
        let mut decrypt_key = security::md5_16_32_32(&session[16..32], client_random, server_random);
        let mut encrypt_key = security::md5_16_32_32(&session[32..48], client_random, server_random);
        if server {
            std::mem::swap(&mut decrypt_key, &mut encrypt_key);
        }

        let key_length = match method {
            ENCRYPTION_METHOD_40BIT | ENCRYPTION_METHOD_56BIT => SHORT_KEY_LENGTH,
            ENCRYPTION_METHOD_128BIT => KEY_LENGTH,
            other => return Err(RdpProtocolError::unsupported(format!("server encryption method not implemented: 0x{:08x}", other))),
        };
        let salt_length = match method {
            ENCRYPTION_METHOD_40BIT => SALT.len(),
            ENCRYPTION_METHOD_56BIT => 1,
            _ => 0,
        };
        sign_key[..salt_length].copy_from_slice(&SALT[..salt_length]);
        decrypt_key[..salt_length].copy_from_slice(&SALT[..salt_length]);
        encrypt_key[..salt_length].copy_from_slice(&SALT[..salt_length]);

        Ok(Self {
            key_length,
            salt_length,
            sign_key,
            encrypt_key,
            decrypt_key,
            encrypt_update_key: encrypt_key,
            decrypt_update_key: decrypt_key,
            encrypt: Rc4Stream::new(&encrypt_key[..key_length])?,
            decrypt: Rc4Stream::new(&decrypt_key[..key_length])?,
            encrypt_use_count: 0,
            decrypt_use_count: 0,
            encrypt_checksum_use_count: 0,
            decrypt_checksum_use_count: 0,
            salted_checksum: true,
        })
    }

    fn update_key(key: &mut [u8; KEY_LENGTH], update_key: &[u8; KEY_LENGTH], key_length: usize, salt_length: usize) -> RdpResult<()> {
        *key = security::key_update_hash(&update_key[..key_length], &key[..key_length]);
        let mut stream = Rc4Stream::new(&key[..key_length])?;
        let mut refreshed = key[..key_length].to_vec();
        stream.apply(&mut refreshed);
        key[..key_length].copy_from_slice(&refreshed);
        key[..salt_length].copy_from_slice(&SALT[..salt_length]);
        Ok(())
    }

    fn sign(&self, data: &[u8], sending: bool, salted: bool) -> RdpResult<[u8; SIGNATURE_LENGTH]> {
        if !salted {
            return Ok(security::mac_signature(&self.sign_key[..self.key_length], data));
        }
        let count = match sending {
            true => self.encrypt_checksum_use_count,
            false => self.decrypt_checksum_use_count.wrapping_sub(1),
        };
        Ok(security::salted_mac_signature(&self.sign_key[..self.key_length], data, count))
    }

    fn encrypt(&mut self, data: &mut [u8]) -> RdpResult<()> {
        if self.encrypt_use_count >= KEY_UPDATE_THRESHOLD {
            Self::update_key(&mut self.encrypt_key, &self.encrypt_update_key, self.key_length, self.salt_length)?;
            self.encrypt = Rc4Stream::new(&self.encrypt_key[..self.key_length])?;
            self.encrypt_use_count = 0;
        }
        self.encrypt.apply(data);
        self.encrypt_use_count += 1;
        self.encrypt_checksum_use_count += 1;
        Ok(())
    }

    fn decrypt(&mut self, data: &mut [u8]) -> RdpResult<()> {
        if self.decrypt_use_count >= KEY_UPDATE_THRESHOLD {
            Self::update_key(&mut self.decrypt_key, &self.decrypt_update_key, self.key_length, self.salt_length)?;
            self.decrypt = Rc4Stream::new(&self.decrypt_key[..self.key_length])?;
            self.decrypt_use_count = 0;
        }
        self.decrypt.apply(data);
        self.decrypt_use_count += 1;
        self.decrypt_checksum_use_count += 1;
        Ok(())
    }
}
