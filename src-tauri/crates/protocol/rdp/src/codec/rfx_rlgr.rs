use crate::codec::bitstream::BitStream;
use crate::core::error::{RdpProtocolError, RdpResult};

const KPMAX: u32 = 80;
const LSGR: u32 = 3;
const UP_GR: u32 = 4;
const DN_GR: u32 = 6;
const UQ_GR: u32 = 3;
const DQ_GR: u32 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RlgrMode {
    Rlgr1,
    Rlgr3,
}

pub fn decode(mode: RlgrMode, source: &[u8], destination: &mut [i16]) -> RdpResult<()> {
    if source.is_empty() || destination.is_empty() {
        return Err(RdpProtocolError::underrun("RLGR input or output is empty".to_string()));
    }

    let mut stream = BitStream::new(source);
    let mut k = 1u32;
    let mut kp = k << LSGR;
    let mut kr = 1u32;
    let mut krp = kr << LSGR;
    let mut written = 0usize;

    while stream.remaining() > 0 && written < destination.len() {
        if k != 0 {
            let mut zeros = stream.count_leading(false);
            if stream.remaining() < 1 {
                break;
            }
            stream.shift(1);

            let mut run = 0usize;
            while zeros > 0 {
                run += 1usize << k;
                kp = (kp + UP_GR).min(KPMAX);
                k = kp >> LSGR;
                zeros -= 1;
            }

            if stream.remaining() < k as usize {
                break;
            }
            run += stream.read_bits(k) as usize;

            if stream.remaining() < 1 {
                break;
            }
            let negative = stream.accumulator() & 0x8000_0000 != 0;
            stream.shift(1);

            let ones = stream.count_leading(true);
            if stream.remaining() < 1 {
                break;
            }
            stream.shift(1);
            if stream.remaining() < kr as usize {
                break;
            }
            let mut code = stream.read_bits(kr) as u16;
            code |= (ones << kr) as u16;
            update_rice(&mut kr, &mut krp, ones);

            kp = kp.saturating_sub(DN_GR);
            k = kp >> LSGR;

            let magnitude = i32::from(code) + 1;
            let run = run.min(destination.len() - written);
            destination[written..written + run].fill(0);
            written += run;
            push(destination, &mut written, if negative { -magnitude } else { magnitude });
            continue;
        }

        let ones = stream.count_leading(true);
        if stream.remaining() < 1 {
            break;
        }
        stream.shift(1);
        if stream.remaining() < kr as usize {
            break;
        }
        let mut code = stream.read_bits(kr) as u16;
        code |= (ones << kr) as u16;
        update_rice(&mut kr, &mut krp, ones);

        match mode {
            RlgrMode::Rlgr1 => match code {
                0 => {
                    kp = (kp + UQ_GR).min(KPMAX);
                    k = kp >> LSGR;
                    push(destination, &mut written, 0);
                }
                _ => {
                    kp = kp.saturating_sub(DQ_GR);
                    k = kp >> LSGR;
                    push(destination, &mut written, zigzag(u32::from(code)));
                }
            },
            RlgrMode::Rlgr3 => {
                let bits = match code {
                    0 => 0,
                    _ => 32 - u32::from(code).leading_zeros(),
                };
                if (stream.remaining() as u32) < bits {
                    break;
                }
                let first = stream.read_bits(bits);
                let second = u32::from(code).wrapping_sub(first);

                if first != 0 && second != 0 {
                    kp = kp.saturating_sub(2 * DQ_GR);
                    k = kp >> LSGR;
                } else if first == 0 && second == 0 {
                    kp = (kp + 2 * UQ_GR).min(KPMAX);
                    k = kp >> LSGR;
                }

                push(destination, &mut written, zigzag(first));
                push(destination, &mut written, zigzag(second));
            }
        }
    }

    destination[written..].fill(0);
    Ok(())
}

fn update_rice(kr: &mut u32, krp: &mut u32, ones: u32) {
    match ones {
        0 => {
            *krp = krp.saturating_sub(2);
            *kr = *krp >> LSGR;
        }
        1 => {}
        _ => {
            *krp = (*krp + ones).min(KPMAX);
            *kr = *krp >> LSGR;
        }
    }
}

fn zigzag(code: u32) -> i32 {
    match code & 1 {
        0 => (code >> 1) as i32,
        _ => -(((code + 1) >> 1) as i32),
    }
}

fn push(destination: &mut [i16], written: &mut usize, value: i32) {
    if *written >= destination.len() {
        return;
    }
    destination[*written] = value as i16;
    *written += 1;
}
