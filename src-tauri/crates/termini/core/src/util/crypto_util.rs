use base64::Engine;
use std::sync::OnceLock;
use anyhow::{bail, Context, Result};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use std::sync::atomic::{AtomicBool, Ordering};
use base64::engine::general_purpose::STANDARD as B64;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const PHRASE_GROUP_BYTES: usize = 4;
pub const ENC_PREFIX: &str = "enc1:";
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const KEYRING_SERVICE: &str = "Termini";
const KEYRING_USER: &str = "db-field-key";

static KEY_BROKEN: AtomicBool = AtomicBool::new(false);

fn cipher() -> &'static Aes256Gcm {
    static CIPHER: OnceLock<Aes256Gcm> = OnceLock::new();
    CIPHER.get_or_init(|| {
        let key = load_or_create_key();
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key))
    })
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn load_or_create_key() -> [u8; KEY_LEN] {
    let work = || {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).expect("failed to open system credential store");
        match entry.get_password() {
            Ok(value) => {
                let bytes = B64.decode(value).expect("failed to decode master key");
                bytes.try_into().expect("master key length is not 32 bytes")
            }
            Err(keyring::Error::NoEntry) => {
                let mut key = [0u8; KEY_LEN];
                OsRng.fill_bytes(&mut key);
                entry.set_password(&B64.encode(key)).expect("failed to write master key to system credential store");
                key
            }
            Err(error) => panic!("failed to read master key: {error}"),
        }
    };
    #[cfg(target_os = "linux")]
    {
        std::thread::Builder::new().name("keyring".into()).spawn(work).expect("spawn keyring").join().unwrap_or_else(|err| std::panic::resume_unwind(err))
    }
    #[cfg(not(target_os = "linux"))]
    work()
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn key_file_path() -> std::path::PathBuf {
    let data_dir = std::env::var("TERMINI_DATA_DIR").expect("TERMINI_DATA_DIR not set");
    if data_dir.is_empty() {
        panic!("TERMINI_DATA_DIR is empty");
    }
    std::path::PathBuf::from(data_dir).join(KEYRING_USER)
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn write_key_file(key: &[u8; KEY_LEN]) -> Result<()> {
    let path = key_file_path();
    let Some(dir) = path.parent() else {
        bail!("master key path has no parent");
    };
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    std::fs::write(&path, B64.encode(key)).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn load_or_create_key() -> [u8; KEY_LEN] {
    let path = key_file_path();
    match std::fs::read_to_string(&path) {
        Ok(value) => {
            let bytes = B64.decode(value).expect("failed to decode master key");
            bytes.try_into().expect("master key length is not 32 bytes")
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut key = [0u8; KEY_LEN];
            OsRng.fill_bytes(&mut key);
            write_key_file(&key).expect("failed to write master key");
            key
        }
        Err(error) => panic!("failed to read master key: {error}"),
    }
}

pub fn key_broken() -> bool {
    KEY_BROKEN.load(Ordering::Acquire)
}

pub fn export_phrase() -> String {
    let key = load_or_create_key();
    let groups: Vec<String> = key.chunks(PHRASE_GROUP_BYTES).map(|chunk| chunk.iter().map(|byte| format!("{byte:02X}")).collect()).collect();
    groups.join("-")
}

pub fn import_phrase(phrase: &str) -> Result<()> {
    let key = parse_phrase(phrase)?;
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        write_key_file(&key)?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).context("failed to open system credential store")?;
        entry.set_password(&B64.encode(key)).context("failed to write master key to system credential store")?;
        Ok(())
    }
}

fn parse_phrase(phrase: &str) -> Result<[u8; KEY_LEN]> {
    let hex: String = phrase.chars().filter(|item| !item.is_whitespace() && *item != '-').collect();
    if hex.len() != KEY_LEN * 2 {
        bail!("recovery phrase must be {} hex characters, got {}", KEY_LEN * 2, hex.len());
    }
    let mut key = [0u8; KEY_LEN];
    for (index, slot) in key.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).context("recovery phrase contains non-hex characters")?;
    }
    Ok(key)
}

pub fn encrypt_text(plain: &str) -> String {
    if plain.is_empty() || plain.starts_with(ENC_PREFIX) {
        return plain.to_string();
    }
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let cipher_text = cipher().encrypt(nonce, plain.as_bytes()).expect("field encryption failed");
    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&cipher_text);
    format!("{}{}", ENC_PREFIX, B64.encode(payload))
}

pub fn decrypt_text(value: &str) -> String {
    let Some(encoded) = value.strip_prefix(ENC_PREFIX) else {
        return value.to_string();
    };
    let payload = B64.decode(encoded).expect("failed to decode encrypted field");
    if payload.len() <= NONCE_LEN {
        panic!("invalid encrypted field length");
    }
    let (nonce_bytes, cipher_text) = payload.split_at(NONCE_LEN);
    let Ok(plain) = cipher().decrypt(Nonce::from_slice(nonce_bytes), cipher_text) else {
        KEY_BROKEN.store(true, Ordering::Release);
        panic!("field decryption failed, the master key in the system credential store may be lost");
    };
    String::from_utf8(plain).expect("decrypted data is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_phrase_accepts_grouped_and_flat() {
        let grouped = "00112233-44556677-8899AABB-CCDDEEFF-00112233-44556677-8899AABB-CCDDEEFF";
        let flat = grouped.replace('-', "");
        assert_eq!(parse_phrase(grouped).unwrap(), parse_phrase(&flat).unwrap());
        assert_eq!(parse_phrase(grouped).unwrap()[0], 0x00);
        assert_eq!(parse_phrase(grouped).unwrap()[31], 0xFF);
    }

    #[test]
    fn parse_phrase_rejects_bad_length_and_charset() {
        assert!(parse_phrase("00112233").is_err());
        assert!(parse_phrase(&"ZZ".repeat(32)).is_err());
    }
}
