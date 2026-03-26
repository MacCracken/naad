//! DSP utility functions: dB conversion, clipping, interpolation.
//!
//! Shared free functions used across synthesis and effects modules.

use serde::{Deserialize, Serialize};

/// Convert linear amplitude to decibels.
///
/// Returns `-f32::INFINITY` for amplitude <= 0.
#[inline]
#[must_use]
pub fn amplitude_to_db(amplitude: f32) -> f32 {
    if amplitude <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * amplitude.log10()
    }
}

/// Convert decibels to linear amplitude.
///
/// Returns 0.0 for `-f32::INFINITY`.
#[inline]
#[must_use]
pub fn db_to_amplitude(db: f32) -> f32 {
    if db == f32::NEG_INFINITY {
        0.0
    } else {
        10.0f32.powf(db / 20.0)
    }
}

/// Normalize a buffer so the peak absolute value is 1.0.
///
/// Does nothing if the buffer is all zeros.
pub fn normalize(buffer: &mut [f32]) {
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak > 0.0 {
        let inv = 1.0 / peak;
        for s in buffer.iter_mut() {
            *s *= inv;
        }
    }
}

/// Hard-limit (clip) a sample to the range \[-limit, +limit\].
#[inline]
#[must_use]
pub fn hard_limit(sample: f32, limit: f32) -> f32 {
    sample.clamp(-limit, limit)
}

/// Soft-clip a sample using `tanh` saturation.
///
/// `drive` controls the amount of saturation (1.0 = mild, higher = more).
#[inline]
#[must_use]
pub fn soft_clip_tanh(sample: f32, drive: f32) -> f32 {
    (sample * drive).tanh()
}

/// Linear interpolation between two values.
#[inline]
#[must_use]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Cubic Hermite interpolation between samples.
///
/// Given four equally-spaced samples `y0, y1, y2, y3` and a fractional
/// position `t` (0..1) between `y1` and `y2`, returns the interpolated value.
#[inline]
#[must_use]
pub fn hermite_interpolate(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + c0
}

/// Crossfade between two signals with equal-power law.
///
/// `mix` ranges from 0.0 (100% dry) to 1.0 (100% wet).
#[inline]
#[must_use]
pub fn crossfade_equal_power(dry: f32, wet: f32, mix: f32) -> f32 {
    let angle = mix * std::f32::consts::FRAC_PI_2;
    dry * angle.cos() + wet * angle.sin()
}

/// Smoothing mode for parameter transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SmoothingMode {
    /// Exponential moving average (one-pole lowpass).
    Exponential,
    /// Linear ramp over a fixed number of samples.
    Linear,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_amplitude_roundtrip() {
        let amp = 0.5;
        let db = amplitude_to_db(amp);
        let back = db_to_amplitude(db);
        assert!(
            (amp - back).abs() < 1e-5,
            "roundtrip failed: {amp} -> {db} -> {back}"
        );
    }

    #[test]
    fn test_db_zero() {
        assert_eq!(amplitude_to_db(1.0), 0.0);
        assert_eq!(db_to_amplitude(0.0), 1.0);
    }

    #[test]
    fn test_db_negative_infinity() {
        assert_eq!(amplitude_to_db(0.0), f32::NEG_INFINITY);
        assert_eq!(db_to_amplitude(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn test_normalize() {
        let mut buf = [0.5, -1.0, 0.25];
        normalize(&mut buf);
        assert!((buf[1].abs() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_silence() {
        let mut buf = [0.0, 0.0, 0.0];
        normalize(&mut buf);
        assert!(buf.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_hard_limit() {
        assert_eq!(hard_limit(2.0, 1.0), 1.0);
        assert_eq!(hard_limit(-2.0, 1.0), -1.0);
        assert_eq!(hard_limit(0.5, 1.0), 0.5);
    }

    #[test]
    fn test_soft_clip() {
        let out = soft_clip_tanh(10.0, 1.0);
        assert!(
            (out - 1.0).abs() < 0.01,
            "tanh(10) should be near 1.0, got {out}"
        );
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 1.0, 0.5) - 0.5).abs() < f32::EPSILON);
        assert!((lerp(0.0, 1.0, 0.0) - 0.0).abs() < f32::EPSILON);
        assert!((lerp(0.0, 1.0, 1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hermite() {
        // For a straight line y = x at points 0,1,2,3, hermite at t=0.5 should be 1.5
        let val = hermite_interpolate(0.0, 1.0, 2.0, 3.0, 0.5);
        assert!((val - 1.5).abs() < 0.01, "hermite on linear data: {val}");
    }

    #[test]
    fn test_crossfade() {
        let dry_only = crossfade_equal_power(1.0, 0.0, 0.0);
        assert!((dry_only - 1.0).abs() < 0.01);
        let wet_only = crossfade_equal_power(0.0, 1.0, 1.0);
        assert!((wet_only - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_serde_roundtrip_smoothing_mode() {
        let mode = SmoothingMode::Exponential;
        let json = serde_json::to_string(&mode).unwrap();
        let back: SmoothingMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }
}
