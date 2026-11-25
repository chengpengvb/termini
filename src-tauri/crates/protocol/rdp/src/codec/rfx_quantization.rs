use crate::codec::rfx_dwt::TILE_COEFFICIENTS;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const QUANT_VALUES: usize = 10;

const BANDS: [(usize, usize, usize); QUANT_VALUES] =
    [(0, 1024, 8), (1024, 1024, 7), (2048, 1024, 9), (3072, 256, 5), (3328, 256, 4), (3584, 256, 6), (3840, 64, 2), (3904, 64, 1), (3968, 64, 3), (4032, 64, 0)];

pub fn decode(buffer: &mut [i16; TILE_COEFFICIENTS], factors: &[u32; QUANT_VALUES]) -> RdpResult<()> {
    if let Some(index) = factors.iter().position(|factor| *factor < 1) {
        return Err(RdpProtocolError::protocol(format!("RemoteFX quantization factor is zero: index={}", index)));
    }

    for (offset, length, factor_index) in BANDS {
        let shift = factors[factor_index] - 1;
        if shift == 0 {
            continue;
        }
        if shift >= 16 {
            return Err(RdpProtocolError::protocol(format!("RemoteFX quantization factor out of range: {}", factors[factor_index])));
        }
        for value in buffer[offset..offset + length].iter_mut() {
            *value = ((u32::from(*value as u16) << shift) & 0xFFFF) as u16 as i16;
        }
    }
    Ok(())
}
