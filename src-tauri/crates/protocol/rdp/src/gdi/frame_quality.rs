pub fn smooth_frame_duration(previous: u64, sample: u64) -> u64 {
    if previous == 0 {
        return sample;
    }
    let delta = previous.abs_diff(sample);
    let step = delta / 8 + u64::from(delta % 8 != 0);
    if sample >= previous {
        previous + step
    } else {
        previous - step
    }
}

pub fn suggest_quality(lag_ms: u64) -> u8 {
    match lag_ms {
        0..=40 => 90,
        41..=100 => 85,
        101..=200 => 80,
        201..=350 => 75,
        351..=600 => 65,
        _ => 55,
    }
}

pub fn suggest_bitrate(lag_ms: u64) -> u32 {
    match lag_ms {
        0..=40 => 12_000_000,
        41..=100 => 10_500_000,
        101..=200 => 8_000_000,
        201..=350 => 5_000_000,
        351..=600 => 3_000_000,
        _ => 1_500_000,
    }
}

pub fn network_bars(lag_ms: u64) -> u8 {
    match lag_ms {
        0..=80 => 4,
        81..=160 => 3,
        161..=300 => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_tracks_sustained_slowdown_and_recovery() {
        let mut value = 20;
        for _ in 0..16 {
            value = smooth_frame_duration(value, 240);
        }
        assert!(value > 200);
        for _ in 0..32 {
            value = smooth_frame_duration(value, 20);
        }
        assert!(value < 30);
        assert_eq!(smooth_frame_duration(u64::MAX, u64::MAX), u64::MAX);
        assert!(smooth_frame_duration(1, u64::MAX) > 1);
    }

    #[test]
    fn moderate_latency_preserves_text_quality_without_hiding_latency() {
        assert_eq!(suggest_quality(80), 85);
        assert!(suggest_bitrate(80) > 10_000_000);
        assert_eq!(suggest_bitrate(1000), 1_500_000);
        assert_eq!(network_bars(400), 1);
        let mut previous = 90;
        for lag in [0, 40, 80, 160, 300, 500, 2000, u64::MAX] {
            let quality = suggest_quality(lag);
            assert!((55..=90).contains(&quality));
            assert!(quality <= previous);
            previous = quality;
        }
    }
}
