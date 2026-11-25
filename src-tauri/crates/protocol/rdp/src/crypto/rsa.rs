use num_bigint::BigUint;
use crate::core::error::{RdpProtocolError, RdpResult};

pub fn public_encrypt(input: &[u8], modulus: &[u8], exponent: &[u8]) -> RdpResult<Vec<u8>> {
    if exponent.is_empty() || modulus.iter().all(|byte| *byte == 0) {
        return Err(RdpProtocolError::protocol(format!("invalid RSA public key: modulus {} bytes exponent {} bytes", modulus.len(), exponent.len())));
    }
    if input.len() > modulus.len() {
        return Err(RdpProtocolError::protocol(format!("RSA plaintext of {} bytes exceeds modulus of {} bytes", input.len(), modulus.len())));
    }
    let value = BigUint::from_bytes_le(input);
    let power = BigUint::from_bytes_le(exponent);
    let base = BigUint::from_bytes_le(modulus);
    let mut output = value.modpow(&power, &base).to_bytes_le();
    output.resize(modulus.len(), 0);
    Ok(output)
}
