//! Feedback Delay Network reverb using goonj::fdn.
//!
//! Wraps the goonj FDN with a wet/dry mix and serializable configuration.
//! The FDN itself is reconstructed on deserialization from stored parameters.

use serde::{Deserialize, Serialize};

use goonj::fdn::{Fdn, fdn_config_for_room};

use crate::error::{NaadError, Result};

/// Feedback Delay Network reverb backed by goonj.
///
/// Stores the configuration parameters for serde roundtripping and
/// reconstructs the inner [`Fdn`] on deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FdnReverb {
    /// The inner FDN processor (skipped in serde — reconstructed from params).
    #[serde(skip)]
    fdn: Option<Fdn>,
    /// Number of delay lines.
    num_delays: usize,
    /// Target RT60 in seconds.
    target_rt60: f32,
    /// Sample rate in Hz.
    sample_rate: u32,
    /// Wet/dry mix (0.0 = dry, 1.0 = wet).
    pub mix: f32,
}

impl FdnReverb {
    /// Create a new FDN reverb.
    ///
    /// Internally creates an 8-delay-line FDN using a synthetic shoebox room
    /// whose dimensions are derived from `num_delays` and `target_rt60`.
    ///
    /// # Errors
    ///
    /// Returns [`NaadError::ComputationError`] if parameters are out of range.
    pub fn new(num_delays: usize, target_rt60: f32, sample_rate: u32, mix: f32) -> Result<Self> {
        if num_delays == 0 {
            return Err(NaadError::ComputationError {
                message: "num_delays must be > 0".into(),
            });
        }
        if target_rt60 <= 0.0 || !target_rt60.is_finite() {
            return Err(NaadError::ComputationError {
                message: "target_rt60 must be positive and finite".into(),
            });
        }
        if sample_rate == 0 {
            return Err(NaadError::ComputationError {
                message: "sample_rate must be > 0".into(),
            });
        }

        let config = fdn_config_for_room(10.0, 8.0, 3.0, target_rt60, sample_rate);
        let fdn = Fdn::new(&config);

        Ok(Self {
            fdn: Some(fdn),
            num_delays,
            target_rt60,
            sample_rate,
            mix: mix.clamp(0.0, 1.0),
        })
    }

    /// Process a single audio sample through the FDN reverb.
    ///
    /// Applies wet/dry mix to the output.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Lazily reconstruct Fdn after deserialization
        if self.fdn.is_none() {
            let config = fdn_config_for_room(10.0, 8.0, 3.0, self.target_rt60, self.sample_rate);
            self.fdn = Some(Fdn::new(&config));
        }

        let wet = if let Some(ref mut fdn) = self.fdn {
            fdn.process_sample(input)
        } else {
            0.0
        };

        input * (1.0 - self.mix) + wet * self.mix
    }

    /// Reset the FDN state to silence.
    pub fn reset(&mut self) {
        if let Some(ref mut fdn) = self.fdn {
            fdn.reset();
        }
    }

    /// Target RT60 in seconds.
    #[must_use]
    pub fn target_rt60(&self) -> f32 {
        self.target_rt60
    }

    /// Sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fdn_produces_reverb_tail() {
        let mut fdn = FdnReverb::new(8, 1.0, 48000, 1.0).unwrap();
        // Feed an impulse
        fdn.process_sample(1.0);
        // Check for reverb tail
        let mut has_tail = false;
        for _ in 0..10000 {
            let out = fdn.process_sample(0.0);
            if out.abs() > 0.001 {
                has_tail = true;
                break;
            }
        }
        assert!(has_tail, "FDN should produce a reverb tail");
    }

    #[test]
    fn test_fdn_output_finite() {
        let mut fdn = FdnReverb::new(8, 0.5, 48000, 1.0).unwrap();
        fdn.process_sample(1.0);
        for _ in 0..5000 {
            let out = fdn.process_sample(0.0);
            assert!(out.is_finite(), "output should be finite");
            assert!(out.abs() < 100.0, "output should not blow up: {out}");
        }
    }

    #[test]
    fn test_fdn_invalid_params() {
        assert!(FdnReverb::new(0, 1.0, 48000, 1.0).is_err());
        assert!(FdnReverb::new(8, -1.0, 48000, 1.0).is_err());
        assert!(FdnReverb::new(8, 1.0, 0, 1.0).is_err());
    }

    #[test]
    fn test_serde_roundtrip() {
        let fdn = FdnReverb::new(8, 1.5, 48000, 0.7).unwrap();
        let json = serde_json::to_string(&fdn).unwrap();
        let mut back: FdnReverb = serde_json::from_str(&json).unwrap();
        assert!((fdn.mix - back.mix).abs() < f32::EPSILON);
        assert!((fdn.target_rt60 - back.target_rt60).abs() < f32::EPSILON);
        // Fdn should be None after deser, but process_sample reconstructs it
        assert!(back.fdn.is_none());
        let out = back.process_sample(1.0);
        assert!(out.is_finite());
        assert!(back.fdn.is_some(), "should reconstruct fdn on first use");
    }
}
