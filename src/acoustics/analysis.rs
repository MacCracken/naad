//! Acoustic analysis metrics from goonj.
//!
//! Wraps goonj's ISO 3382-1 analysis functions to compute room acoustic
//! metrics from an impulse response: clarity (C50, C80), definition (D50),
//! speech transmission index (STI), and reverberation time (RT60).

use serde::{Deserialize, Serialize};

use goonj::analysis::{clarity_c50, clarity_c80, definition_d50, sti_estimate};
use goonj::impulse::ImpulseResponse;

use crate::error::{NaadError, Result};

/// Room acoustic metrics computed from an impulse response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMetrics {
    /// Clarity C50 in dB — early (0-50ms) to late energy ratio.
    pub c50: f32,
    /// Clarity C80 in dB — early (0-80ms) to late energy ratio.
    pub c80: f32,
    /// Definition D50 — fraction of energy in first 50ms (0.0-1.0).
    pub d50: f32,
    /// Speech Transmission Index (0.0-1.0).
    pub sti: f32,
    /// Reverberation time RT60 in seconds.
    pub rt60: f32,
}

/// Analyze an impulse response and compute room acoustic metrics.
///
/// Wraps goonj analysis functions to produce a complete set of metrics
/// from a broadband impulse response.
///
/// # Errors
///
/// Returns [`NaadError::ComputationError`] if the impulse response is empty
/// or the sample rate is zero.
pub fn analyze_impulse_response(ir: &[f32], sample_rate: u32) -> Result<RoomMetrics> {
    if ir.is_empty() {
        return Err(NaadError::ComputationError {
            message: "impulse response is empty".into(),
        });
    }
    if sample_rate == 0 {
        return Err(NaadError::ComputationError {
            message: "sample rate must be > 0".into(),
        });
    }

    let goonj_ir = ImpulseResponse {
        samples: ir.to_vec(),
        sample_rate,
        rt60: 0.0, // will be computed below
    };

    let c50 = clarity_c50(&goonj_ir);
    let c80 = clarity_c80(&goonj_ir);
    let d50 = definition_d50(&goonj_ir);
    let sti = sti_estimate(&goonj_ir);
    let rt60 = estimate_rt60(ir, sample_rate);

    Ok(RoomMetrics {
        c50,
        c80,
        d50,
        sti,
        rt60,
    })
}

/// Quick RT60 estimate from an impulse response using Sabine's equation.
///
/// Estimates absorption from the energy decay of the IR and applies
/// Sabine's formula. For a more accurate result, use the full
/// [`analyze_impulse_response`] function.
#[must_use]
#[inline]
pub fn estimate_rt60(ir: &[f32], sample_rate: u32) -> f32 {
    if ir.is_empty() || sample_rate == 0 {
        return 0.0;
    }

    // Estimate RT60 from energy decay curve
    // Find the time at which energy drops by 60 dB
    let total_energy: f32 = ir.iter().map(|&s| s * s).sum();
    if total_energy < f32::EPSILON {
        return 0.0;
    }

    let mut cumulative = 0.0_f32;
    let threshold = total_energy * 0.001; // -30 dB point (we'll double for RT60)

    for (i, &s) in ir.iter().enumerate() {
        cumulative += s * s;
        if cumulative >= total_energy - threshold {
            // Time to reach -30 dB, double for RT60 estimate
            let t30 = i as f32 / sample_rate as f32;
            return t30 * 2.0;
        }
    }

    // Fallback: use full IR length as upper bound
    ir.len() as f32 / sample_rate as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a synthetic exponentially decaying impulse response.
    fn synthetic_ir(sample_rate: u32, duration_secs: f32, decay_rate: f32) -> Vec<f32> {
        let len = (sample_rate as f32 * duration_secs) as usize;
        (0..len)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (-decay_rate * t).exp()
            })
            .collect()
    }

    #[test]
    fn test_analyze_synthetic_ir() {
        let ir = synthetic_ir(48000, 1.0, 5.0);
        let metrics = analyze_impulse_response(&ir, 48000).unwrap();
        assert!(metrics.c50.is_finite(), "C50 should be finite");
        assert!(metrics.c80.is_finite(), "C80 should be finite");
        assert!(metrics.d50.is_finite(), "D50 should be finite");
        assert!(metrics.sti.is_finite(), "STI should be finite");
        assert!(metrics.rt60.is_finite(), "RT60 should be finite");
        assert!(metrics.rt60 > 0.0, "RT60 should be positive");
    }

    #[test]
    fn test_empty_ir_errors() {
        assert!(analyze_impulse_response(&[], 48000).is_err());
    }

    #[test]
    fn test_zero_sample_rate_errors() {
        assert!(analyze_impulse_response(&[1.0, 0.5], 0).is_err());
    }

    #[test]
    fn test_estimate_rt60_basic() {
        let ir = synthetic_ir(48000, 2.0, 3.0);
        let rt60 = estimate_rt60(&ir, 48000);
        assert!(rt60 > 0.0, "RT60 should be positive");
        assert!(rt60.is_finite(), "RT60 should be finite");
    }

    #[test]
    fn test_estimate_rt60_empty() {
        assert_eq!(estimate_rt60(&[], 48000), 0.0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let ir = synthetic_ir(48000, 0.5, 5.0);
        let metrics = analyze_impulse_response(&ir, 48000).unwrap();
        let json = serde_json::to_string(&metrics).unwrap();
        let back: RoomMetrics = serde_json::from_str(&json).unwrap();
        assert!((metrics.c50 - back.c50).abs() < f32::EPSILON);
        assert!((metrics.sti - back.sti).abs() < f32::EPSILON);
    }
}
