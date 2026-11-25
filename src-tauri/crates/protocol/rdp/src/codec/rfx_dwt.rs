pub const TILE_COEFFICIENTS: usize = 4096;

pub fn decode_2d(buffer: &mut [i16; TILE_COEFFICIENTS], scratch: &mut [i16; TILE_COEFFICIENTS]) {
    decode_block(&mut buffer[3840..], scratch, 8);
    decode_block(&mut buffer[3072..], scratch, 16);
    decode_block(&mut buffer[..], scratch, 32);
}

fn decode_block(buffer: &mut [i16], scratch: &mut [i16], subband_width: usize) {
    let total_width = subband_width << 1;
    let band = subband_width * subband_width;

    for y in 0..subband_width {
        let ll = band * 3 + y * subband_width;
        let hl = y * subband_width;
        let lh = band + y * subband_width;
        let hh = band * 2 + y * subband_width;
        let low = y * total_width;
        let high = band * 2 + y * total_width;

        scratch[low] = even(buffer[ll], buffer[hl], buffer[hl]);
        scratch[high] = even(buffer[lh], buffer[hh], buffer[hh]);
        for n in 1..subband_width {
            scratch[low + (n << 1)] = even(buffer[ll + n], buffer[hl + n - 1], buffer[hl + n]);
            scratch[high + (n << 1)] = even(buffer[lh + n], buffer[hh + n - 1], buffer[hh + n]);
        }

        for n in 0..subband_width - 1 {
            let x = n << 1;
            scratch[low + x + 1] = odd(buffer[hl + n], scratch[low + x], scratch[low + x + 2]);
            scratch[high + x + 1] = odd(buffer[hh + n], scratch[high + x], scratch[high + x + 2]);
        }
        let last = subband_width - 1;
        let x = last << 1;
        scratch[low + x + 1] = odd(buffer[hl + last], scratch[low + x], scratch[low + x]);
        scratch[high + x + 1] = odd(buffer[hh + last], scratch[high + x], scratch[high + x]);
    }

    for x in 0..total_width {
        let mut low = x;
        let mut high = x + subband_width * total_width;
        let mut target = x;

        buffer[target] = even(scratch[low], scratch[high], scratch[high]);
        for _ in 1..subband_width {
            low += total_width;
            high += total_width;
            buffer[target + total_width * 2] = even(scratch[low], scratch[high - total_width], scratch[high]);
            buffer[target + total_width] = odd(scratch[high - total_width], buffer[target], buffer[target + total_width * 2]);
            target += total_width * 2;
        }
        buffer[target + total_width] = odd(scratch[high], buffer[target], buffer[target]);
    }
}

fn even(low: i16, previous: i16, current: i16) -> i16 {
    (i32::from(low) - ((i32::from(previous) + i32::from(current) + 1) >> 1)) as i16
}

fn odd(high: i16, previous: i16, next: i16) -> i16 {
    ((i32::from(high) << 1) + ((i32::from(previous) + i32::from(next)) >> 1)) as i16
}
