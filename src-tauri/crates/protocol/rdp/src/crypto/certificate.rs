use sha2::{Digest, Sha256};
use crate::core::stream::ReadCursor;
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::core::der::{DerReader, TAG_BIT_STRING, TAG_INTEGER, TAG_SEQUENCE};

const TAG_VERSION_CONTEXT: u8 = 0xA0;

const CERT_CHAIN_VERSION_MASK: u32 = 0x7FFF_FFFF;
const CERT_CHAIN_VERSION_1: u32 = 0x0000_0001;
const CERT_CHAIN_VERSION_2: u32 = 0x0000_0002;

const SIGNATURE_ALG_RSA: u32 = 0x0000_0001;
const KEY_EXCHANGE_ALG_RSA: u32 = 0x0000_0001;
const BB_RSA_KEY_BLOB: u16 = 0x0006;
const RSA_MAGIC: &[u8] = b"RSA1";
const RSA_PADDING_LENGTH: usize = 8;
const EXPONENT_LENGTH: usize = 4;

pub struct RsaPublicKey {
    pub modulus: Vec<u8>,
    pub exponent: Vec<u8>,
}

pub fn extract_public_key_der(certificate: &[u8]) -> RdpResult<Vec<u8>> {
    let mut outer = DerReader::new(certificate);
    let container = outer.expect(TAG_SEQUENCE)?;
    let mut certificate_reader = DerReader::new(container.content);
    let tbs = certificate_reader.expect(TAG_SEQUENCE)?;
    let mut fields = DerReader::new(tbs.content);
    if fields.peek_tag() == Some(TAG_VERSION_CONTEXT) {
        fields.next_element()?;
    }
    for _ in 0..5 {
        fields.next_element()?;
    }
    let spki = fields.expect(TAG_SEQUENCE)?;
    Ok(spki.raw.to_vec())
}

pub fn extract_public_key_pkcs1(certificate: &[u8]) -> RdpResult<Vec<u8>> {
    let spki = extract_public_key_der(certificate)?;
    let mut reader = DerReader::new(&spki);
    let container = reader.expect(TAG_SEQUENCE)?;
    let mut fields = DerReader::new(container.content);
    fields.next_element()?;
    let key = fields.expect(TAG_BIT_STRING)?;
    let (padding, body) = key.content.split_first().ok_or_else(|| RdpProtocolError::encoding("x509 public key bit string is empty".to_string()))?;
    if *padding != 0 {
        return Err(RdpProtocolError::encoding(format!("invalid x509 public key bit string padding: {}", padding)));
    }
    Ok(body.to_vec())
}

pub fn parse_server_certificate(blob: &[u8]) -> RdpResult<RsaPublicKey> {
    let mut cursor = ReadCursor::new(blob);
    let version = cursor.read_u32_le()?;
    match version & CERT_CHAIN_VERSION_MASK {
        CERT_CHAIN_VERSION_1 => read_proprietary_certificate(&mut cursor),
        CERT_CHAIN_VERSION_2 => read_certificate_chain(&mut cursor),
        other => Err(RdpProtocolError::protocol(format!("invalid server certificate chain version: {}", other))),
    }
}

fn read_proprietary_certificate(cursor: &mut ReadCursor<'_>) -> RdpResult<RsaPublicKey> {
    let signature_algorithm = cursor.read_u32_le()?;
    let key_algorithm = cursor.read_u32_le()?;
    if signature_algorithm != SIGNATURE_ALG_RSA || key_algorithm != KEY_EXCHANGE_ALG_RSA {
        return Err(RdpProtocolError::unsupported(format!("unsupported server certificate algorithm: signature {} key {}", signature_algorithm, key_algorithm)));
    }
    let blob_type = cursor.read_u16_le()?;
    if blob_type != BB_RSA_KEY_BLOB {
        return Err(RdpProtocolError::protocol(format!("invalid server public key blob type: 0x{:04x}", blob_type)));
    }
    let blob_length = usize::from(cursor.read_u16_le()?);
    let mut blob = cursor.sub_cursor(blob_length)?;

    if blob.read_slice(RSA_MAGIC.len())? != RSA_MAGIC {
        return Err(RdpProtocolError::protocol("server public key is missing the RSA1 magic".to_string()));
    }
    let key_length = blob.read_u32_le()? as usize;
    let bit_length = blob.read_u32_le()? as usize;
    let data_length = blob.read_u32_le()? as usize;
    let exponent = blob.read_vec(EXPONENT_LENGTH)?;
    if key_length <= RSA_PADDING_LENGTH {
        return Err(RdpProtocolError::protocol(format!("invalid server public key length: {}", key_length)));
    }
    if key_length != bit_length / 8 + RSA_PADDING_LENGTH {
        return Err(RdpProtocolError::protocol(format!("invalid server public key bit length: {} bits for {} bytes", bit_length, key_length)));
    }
    if data_length != bit_length / 8 - 1 {
        return Err(RdpProtocolError::protocol(format!("invalid server public key data length: {} bits for {} bytes", bit_length, data_length)));
    }
    let modulus = blob.read_vec(key_length - RSA_PADDING_LENGTH)?;
    blob.skip(RSA_PADDING_LENGTH)?;
    Ok(RsaPublicKey { modulus, exponent })
}

fn read_certificate_chain(cursor: &mut ReadCursor<'_>) -> RdpResult<RsaPublicKey> {
    let count = cursor.read_u32_le()?;
    if count == 0 {
        return Err(RdpProtocolError::protocol("server certificate chain is empty".to_string()));
    }
    let mut terminal = &[][..];
    for _ in 0..count {
        let length = cursor.read_u32_le()? as usize;
        terminal = cursor.read_slice(length)?;
    }
    read_x509_public_key(terminal)
}

fn read_x509_public_key(certificate: &[u8]) -> RdpResult<RsaPublicKey> {
    let body = extract_public_key_pkcs1(certificate)?;
    let mut key_reader = DerReader::new(&body);
    let rsa = key_reader.expect(TAG_SEQUENCE)?;
    let mut parts = DerReader::new(rsa.content);
    let modulus = parts.expect(TAG_INTEGER)?;
    let exponent = parts.expect(TAG_INTEGER)?;
    Ok(RsaPublicKey { modulus: to_little_endian(modulus.content), exponent: to_little_endian(exponent.content) })
}

fn to_little_endian(value: &[u8]) -> Vec<u8> {
    let start = value.iter().position(|byte| *byte != 0).unwrap_or(value.len());
    value[start..].iter().rev().copied().collect()
}

pub fn fingerprint_sha256(certificate: &[u8]) -> String {
    let digest = Sha256::digest(certificate);
    digest.iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<String>>().join(":")
}

pub fn ensure_non_empty(certificates: &[Vec<u8>]) -> RdpResult<&Vec<u8>> {
    certificates.first().ok_or_else(|| RdpProtocolError::protocol("server did not present a TLS certificate".to_string()))
}
