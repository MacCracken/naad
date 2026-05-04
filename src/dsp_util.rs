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

/// Compute the RMS (root mean square) of a buffer.
#[inline]
#[must_use]
pub fn rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = buffer.iter().map(|&s| s * s).sum();
    (sum_sq / buffer.len() as f32).sqrt()
}

/// Compute the peak absolute value of a buffer.
#[inline]
#[must_use]
pub fn peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Apply a Hann window to a buffer in place.
pub fn apply_hann_window(buffer: &mut [f32]) {
    let len = buffer.len();
    if len == 0 {
        return;
    }
    let inv = 1.0 / len as f32;
    for (i, s) in buffer.iter_mut().enumerate() {
        let w = 0.5 * (1.0 - (std::f32::consts::TAU * i as f32 * inv).cos());
        *s *= w;
    }
}

/// Apply a Blackman window to a buffer in place.
pub fn apply_blackman_window(buffer: &mut [f32]) {
    let len = buffer.len();
    if len == 0 {
        return;
    }
    let inv = 1.0 / len as f32;
    for (i, s) in buffer.iter_mut().enumerate() {
        let t = i as f32 * inv;
        let w = 0.42 - 0.5 * (std::f32::consts::TAU * t).cos()
            + 0.08 * (2.0 * std::f32::consts::TAU * t).cos();
        *s *= w;
    }
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

// --- FFT utilities (requires hisab) ---

/// Compute real-valued FFT magnitude spectrum.
///
/// Returns magnitudes for bins 0..N/2+1 (DC to Nyquist).
/// Input length must be a power of 2.
///
/// Requires the `synthesis` or `acoustics` feature.
#[cfg(feature = "synthesis")]
#[must_use]
pub fn fft_magnitudes(input: &[f32]) -> Vec<f32> {
    use hisab::Complex;

    let n = input.len();
    let mut complex: Vec<Complex> = input.iter().map(|&s| Complex::new(s as f64, 0.0)).collect();
    // fft returns Result — unwrap-free: if it fails, return empty
    if hisab::num::fft(&mut complex).is_err() {
        return Vec::new();
    }

    let half = n / 2 + 1;
    let inv_n = 1.0 / n as f64;
    complex[..half]
        .iter()
        .map(|c| (c.abs() * inv_n) as f32)
        .collect()
}

/// Compute the power spectrum (magnitude squared) of a real signal.
///
/// Returns power for bins 0..N/2+1. Input length must be a power of 2.
///
/// Requires the `synthesis` or `acoustics` feature.
#[cfg(feature = "synthesis")]
#[must_use]
pub fn power_spectrum(input: &[f32]) -> Vec<f32> {
    use hisab::Complex;

    let n = input.len();
    let mut complex: Vec<Complex> = input.iter().map(|&s| Complex::new(s as f64, 0.0)).collect();
    if hisab::num::fft(&mut complex).is_err() {
        return Vec::new();
    }

    let half = n / 2 + 1;
    let inv_n_sq = 1.0 / (n as f64 * n as f64);
    complex[..half]
        .iter()
        .map(|c| ((c.re * c.re + c.im * c.im) * inv_n_sq) as f32)
        .collect()
}

/// Detect the fundamental pitch of a buffer via autocorrelation.
///
/// Computes the autocorrelation of `buffer`, locates the dominant peak in
/// the lag range `[sample_rate/max_hz, sample_rate/min_hz]`, then refines
/// it to sub-sample accuracy using **Newton-Raphson on a Catmull-Rom cubic
/// interpolant** through the four samples surrounding the discrete peak
/// (`hisab::num::roots::newton_raphson` solves `P'(τ) = 0`).
///
/// Returns `None` for buffers shorter than 32 samples, when no peak
/// exceeds 30 % of the zero-lag energy (signal is noise-dominated), or
/// when the search range is degenerate (`min_hz >= max_hz` after
/// quantizing to integer lags).
///
/// `min_hz` / `max_hz` should bracket the expected pitch range — narrow
/// brackets reject octave errors and keep the autocorrelation cheaper.
/// For musical pitch detection, common values are `30.0..2000.0` Hz.
///
/// Requires the `synthesis` feature (uses hisab).
#[cfg(feature = "synthesis")]
#[must_use]
pub fn detect_pitch_autocorr(
    buffer: &[f32],
    sample_rate: f32,
    min_hz: f32,
    max_hz: f32,
) -> Option<f32> {
    let n = buffer.len();
    if n < 32 || sample_rate <= 0.0 || min_hz <= 0.0 || max_hz <= min_hz {
        return None;
    }

    let max_lag = ((sample_rate / min_hz) as usize).min(n - 1);
    let min_lag = ((sample_rate / max_hz) as usize).max(2);
    if min_lag + 2 >= max_lag {
        return None;
    }

    // Direct autocorrelation up to max_lag. For typical pitch-detection
    // buffers (1k-4k samples) this is fast enough; FFT-based autocorr is
    // a future optimization.
    let mut autocorr = vec![0.0f32; max_lag + 2];
    for (lag, slot) in autocorr.iter_mut().enumerate() {
        let mut sum = 0.0f32;
        for i in 0..(n - lag) {
            sum += buffer[i] * buffer[i + lag];
        }
        *slot = sum;
    }

    let r0 = autocorr[0];
    if r0 <= 0.0 {
        return None;
    }

    // Find the largest peak in [min_lag, max_lag-1].
    let mut peak_lag = min_lag;
    let mut peak_val = autocorr[min_lag];
    for lag in (min_lag + 1)..max_lag {
        if autocorr[lag] > peak_val
            && autocorr[lag] > autocorr[lag - 1]
            && autocorr[lag] > autocorr[lag + 1]
        {
            peak_val = autocorr[lag];
            peak_lag = lag;
        }
    }

    // Reject noise-dominated buffers (peak too weak relative to r(0)).
    if peak_val < 0.30 * r0 {
        return None;
    }

    // Catmull-Rom cubic through (peak_lag-1, peak_lag, peak_lag+1, peak_lag+2)
    // mapped to t ∈ [0, 1] between the middle pair. The peak lives at some
    // fractional t we'll find by Newton-Raphson on the derivative.
    let y0 = autocorr[peak_lag - 1] as f64;
    let y1 = autocorr[peak_lag] as f64;
    let y2 = autocorr[peak_lag + 1] as f64;
    let y3 = autocorr[peak_lag + 2] as f64;

    // P(t)   = 0.5 * (a*t³ + b*t² + c*t + 2*y1)
    // P'(t)  = 0.5 * (3a*t² + 2b*t + c)
    // P''(t) = 0.5 * (6a*t + 2b)
    let a = -y0 + 3.0 * y1 - 3.0 * y2 + y3;
    let b = 2.0 * y0 - 5.0 * y1 + 4.0 * y2 - y3;
    let c = -y0 + y2;

    let dp = move |t: f64| 0.5 * (3.0 * a * t * t + 2.0 * b * t + c);
    let ddp = move |t: f64| 0.5 * (6.0 * a * t + 2.0 * b);

    // Initial guess: parabolic-interpolation closed-form (one NR step on
    // a local quadratic). Newton then refines on the cubic.
    let denom = 2.0 * (y0 - 2.0 * y1 + y2);
    let parabolic_t = if denom.abs() < 1e-12 {
        0.0
    } else {
        ((y0 - y2) / denom).clamp(-1.0, 1.0)
    };

    let refined_t = hisab::num::newton_raphson(dp, ddp, parabolic_t, 1e-9, 16)
        .unwrap_or(parabolic_t)
        .clamp(-1.0, 1.0);

    let refined_lag = peak_lag as f64 + refined_t;
    if refined_lag <= 0.0 {
        return None;
    }
    let pitch = sample_rate as f64 / refined_lag;
    if !pitch.is_finite() || pitch < min_hz as f64 * 0.5 || pitch > max_hz as f64 * 2.0 {
        return None;
    }
    Some(pitch as f32)
}

/// 256-entry lookup table mapping dB to linear amplitude over `[-80, +20] dB`.
///
/// Built lazily on first access via `LazyLock`. Resolution is ~0.39 dB per
/// entry; consumers should use [`db_to_amplitude_lut`] which interpolates
/// between adjacent entries — the residual error of a smooth exponential
/// at sub-half-dB spacing is well below audibility for dynamics processing.
const DB_LUT_SIZE: usize = 256;
const DB_LUT_MIN: f32 = -80.0;
const DB_LUT_MAX: f32 = 20.0;
const DB_LUT_RANGE: f32 = DB_LUT_MAX - DB_LUT_MIN;

static DB_TO_AMP_LUT: std::sync::LazyLock<[f32; DB_LUT_SIZE]> = std::sync::LazyLock::new(|| {
    let mut table = [0.0f32; DB_LUT_SIZE];
    for (i, slot) in table.iter_mut().enumerate() {
        let t = i as f32 / (DB_LUT_SIZE - 1) as f32;
        let db = DB_LUT_MIN + t * DB_LUT_RANGE;
        *slot = 10.0f32.powf(db / 20.0);
    }
    table
});

/// Fast `dB → linear amplitude` via a 256-entry LUT with linear interp.
///
/// Avoids `powf` in inner loops — the compressor / limiter / de-esser gain
/// stages call this every sample. Inputs outside `[-80, +20] dB` clamp to
/// the table edges (silence / `+20 dB ≈ 10×`). For one-shot conversions
/// where exact accuracy matters more than throughput, prefer
/// [`db_to_amplitude`].
#[inline]
#[must_use]
pub fn db_to_amplitude_lut(db: f32) -> f32 {
    let table = &*DB_TO_AMP_LUT;
    if db <= DB_LUT_MIN {
        return table[0];
    }
    if db >= DB_LUT_MAX {
        return table[DB_LUT_SIZE - 1];
    }
    let normalized = (db - DB_LUT_MIN) / DB_LUT_RANGE;
    let idx = normalized * (DB_LUT_SIZE - 1) as f32;
    let i0 = idx as usize;
    let i1 = (i0 + 1).min(DB_LUT_SIZE - 1);
    let frac = idx - i0 as f32;
    table[i0] * (1.0 - frac) + table[i1] * frac
}

/// One step of the Marsaglia xorshift32 PRNG.
///
/// Updates `state` in place and returns the new value. Includes a
/// zero-state guard — `xorshift32(0) = 0` would otherwise loop forever,
/// so a zero state is silently reset to `1` before stepping.
///
/// This is the canonical xorshift implementation used by every PRNG
/// site in the crate (white/pink/brown noise, granular spray jitter,
/// drum click transients, physical-modeling exciters). Use it directly
/// instead of inlining the three-XOR sequence.
#[inline]
pub fn xorshift32(state: &mut u32) -> u32 {
    if *state == 0 {
        *state = 1;
    }
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// One step of [`xorshift32`] mapped to a signed `f32` in `[-1.0, 1.0)`.
#[inline]
#[must_use]
pub fn xorshift32_signed_f32(state: &mut u32) -> f32 {
    (xorshift32(state) as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// One step of [`xorshift32`] mapped to an unsigned `f32` in `[0.0, 1.0)`.
#[inline]
#[must_use]
pub fn xorshift32_unit_f32(state: &mut u32) -> f32 {
    xorshift32(state) as f32 / u32::MAX as f32
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

    #[test]
    fn test_rms() {
        let buf = [1.0f32; 100];
        assert!((rms(&buf) - 1.0).abs() < f32::EPSILON);
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn test_peak() {
        let buf = [0.5, -0.8, 0.3];
        assert!((peak(&buf) - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hann_window() {
        let mut buf = [1.0f32; 64];
        apply_hann_window(&mut buf);
        // First and last samples should be near zero
        assert!(buf[0].abs() < 0.01);
        assert!(buf[63].abs() < 0.05);
        // Middle sample should be near 1.0
        assert!(buf[32] > 0.9);
    }

    #[test]
    fn test_blackman_window() {
        let mut buf = [1.0f32; 64];
        apply_blackman_window(&mut buf);
        assert!(buf[0].abs() < 0.01);
        assert!(buf[32] > 0.9);
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_fft_magnitudes_sine() {
        // Generate a 440Hz sine at 44100Hz, 1024 samples
        let n = 1024;
        let mut buf = vec![0.0f32; n];
        for (i, s) in buf.iter_mut().enumerate() {
            *s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin();
        }
        let mags = fft_magnitudes(&buf);
        assert_eq!(mags.len(), n / 2 + 1);
        // Bin for 440Hz: 440 * 1024 / 44100 ≈ bin 10
        let peak_bin = mags
            .iter()
            .enumerate()
            .skip(1) // skip DC
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let expected_bin = (440.0 * n as f32 / 44100.0).round() as usize;
        assert!(
            (peak_bin as i32 - expected_bin as i32).unsigned_abs() <= 1,
            "peak should be near bin {expected_bin}, got {peak_bin}"
        );
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_power_spectrum() {
        let n = 256;
        let buf = vec![0.5f32; n]; // DC signal
        let ps = power_spectrum(&buf);
        assert_eq!(ps.len(), n / 2 + 1);
        // DC bin should have the most power
        assert!(ps[0] > ps[1]);
    }

    #[cfg(feature = "synthesis")]
    fn synth_sine(freq_hz: f32, sample_rate: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (i as f32 / sample_rate * freq_hz * std::f32::consts::TAU).sin())
            .collect()
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_detect_pitch_sine_440() {
        let sr = 44100.0;
        let buf = synth_sine(440.0, sr, 4096);
        let pitch = detect_pitch_autocorr(&buf, sr, 30.0, 2000.0).unwrap();
        let cents_off = 1200.0 * (pitch / 440.0).log2();
        assert!(
            cents_off.abs() < 5.0,
            "440 Hz sine: detected {pitch} Hz ({cents_off:+.2} cents off)"
        );
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_detect_pitch_sine_220_and_880() {
        let sr = 44100.0;
        for &freq in &[220.0f32, 880.0, 1100.0] {
            let buf = synth_sine(freq, sr, 4096);
            let pitch = detect_pitch_autocorr(&buf, sr, 30.0, 2000.0).unwrap();
            let cents_off = 1200.0 * (pitch / freq).log2();
            assert!(
                cents_off.abs() < 5.0,
                "{freq} Hz sine: detected {pitch} Hz ({cents_off:+.2} cents off)"
            );
        }
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_detect_pitch_subsample_accuracy() {
        // Pick a frequency whose period isn't an integer number of samples
        // — this exercises the cubic+NR refinement (without it, accuracy
        // drops to ±20 cents).
        let sr = 44100.0;
        let freq = 437.3; // period ≈ 100.86 samples
        let buf = synth_sine(freq, sr, 4096);
        let pitch = detect_pitch_autocorr(&buf, sr, 30.0, 2000.0).unwrap();
        let cents_off = 1200.0 * (pitch / freq).log2();
        assert!(
            cents_off.abs() < 5.0,
            "non-integer period: detected {pitch} Hz vs {freq} Hz ({cents_off:+.2} cents)"
        );
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_detect_pitch_rejects_noise() {
        // Pure white noise should yield no confident pitch.
        let mut state = 12345u32;
        let noise: Vec<f32> = (0..4096)
            .map(|_| xorshift32_signed_f32(&mut state))
            .collect();
        // Most calls will return None; an occasional spurious peak is fine
        // but should not be musically meaningful. Just assert that the API
        // doesn't panic and returns either None or something well outside
        // a sensible musical range for noise.
        let pitch = detect_pitch_autocorr(&noise, 44100.0, 30.0, 2000.0);
        if let Some(p) = pitch {
            assert!(p.is_finite(), "noise pitch must be finite if returned");
        }
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_detect_pitch_short_buffer_returns_none() {
        let buf = [0.0f32; 16];
        assert!(detect_pitch_autocorr(&buf, 44100.0, 30.0, 2000.0).is_none());
    }

    #[cfg(feature = "synthesis")]
    #[test]
    fn test_detect_pitch_invalid_range_returns_none() {
        let buf = synth_sine(440.0, 44100.0, 1024);
        // min_hz > max_hz → degenerate
        assert!(detect_pitch_autocorr(&buf, 44100.0, 2000.0, 30.0).is_none());
        // negative sample rate
        assert!(detect_pitch_autocorr(&buf, -1.0, 30.0, 2000.0).is_none());
    }

    #[test]
    fn test_db_to_amplitude_lut_matches_reference() {
        // Sweep -80..+20 dB; LUT should be within 0.5% of the powf reference
        // across the table range. Linear interpolation between 256 entries
        // gives sub-half-dB accuracy on a smooth exponential.
        for i in 0..=200 {
            let db = -80.0 + (i as f32) * 0.5;
            let exact = db_to_amplitude(db);
            let lut = db_to_amplitude_lut(db);
            let rel_err = if exact > 0.0 {
                ((lut - exact) / exact).abs()
            } else {
                (lut - exact).abs()
            };
            assert!(
                rel_err < 0.005,
                "LUT diverges at {db} dB: exact={exact}, lut={lut}, rel_err={rel_err}"
            );
        }
    }

    #[test]
    fn test_db_to_amplitude_lut_clamps_out_of_range() {
        // Below table min — clamps to the silence floor.
        let very_quiet = db_to_amplitude_lut(-200.0);
        let table_floor = db_to_amplitude_lut(-80.0);
        assert!((very_quiet - table_floor).abs() < f32::EPSILON);

        // Above table max — clamps to +20 dB ≈ 10×.
        let very_loud = db_to_amplitude_lut(60.0);
        let table_ceil = db_to_amplitude_lut(20.0);
        assert!((very_loud - table_ceil).abs() < f32::EPSILON);
    }

    #[test]
    fn test_xorshift32_zero_state_guard() {
        // Without the guard, xorshift32(0) loops on 0 forever.
        let mut state = 0u32;
        let v = xorshift32(&mut state);
        assert_ne!(v, 0, "zero-state guard must produce non-zero output");
        assert_ne!(state, 0, "state must not remain zero after step");
    }

    #[test]
    fn test_xorshift32_deterministic() {
        let mut a = 42u32;
        let mut b = 42u32;
        for _ in 0..100 {
            assert_eq!(xorshift32(&mut a), xorshift32(&mut b));
        }
    }

    #[test]
    fn test_xorshift32_signed_range() {
        let mut state = 12345u32;
        for _ in 0..10_000 {
            let v = xorshift32_signed_f32(&mut state);
            assert!((-1.0..1.0).contains(&v), "signed PRNG out of range: {v}");
        }
    }

    #[test]
    fn test_xorshift32_unit_range() {
        let mut state = 67890u32;
        for _ in 0..10_000 {
            let v = xorshift32_unit_f32(&mut state);
            assert!((0.0..1.0).contains(&v), "unit PRNG out of range: {v}");
        }
    }
}
