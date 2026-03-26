//! Additive synthesis with per-partial control.
//!
//! Builds complex timbres by summing individually controllable sine
//! partials. Each partial has an independent frequency ratio (relative
//! to a fundamental) and amplitude. The default constructor creates a
//! harmonic series with 1/n amplitude rolloff.

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Maximum number of partials supported.
const MAX_PARTIALS: usize = 64;

/// A single partial: frequency ratio and amplitude.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Partial {
    /// Frequency multiplier relative to the fundamental.
    pub frequency_ratio: f32,
    /// Amplitude (0.0 to 1.0).
    pub amplitude: f32,
    /// Phase offset (0..1).
    pub phase: f32,
}

/// Additive synthesis engine.
///
/// Sums up to 64 sine-wave partials, each at an independent frequency
/// ratio relative to the fundamental.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditiveSynth {
    /// Fundamental frequency in Hz.
    fundamental: f32,
    /// Partial definitions.
    partials: Vec<Partial>,
    /// Running phases for each partial (0..1).
    phases: Vec<f32>,
    /// Sample rate in Hz.
    sample_rate: f32,
}

impl AdditiveSynth {
    /// Create a new additive synth with a harmonic series.
    ///
    /// Generates `num_partials` harmonics at 1x, 2x, 3x ... with
    /// amplitudes of 1/1, 1/2, 1/3 ... (normalized).
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid.
    pub fn new(fundamental: f32, num_partials: usize, sample_rate: f32) -> Result<Self> {
        if sample_rate <= 0.0 || !sample_rate.is_finite() {
            return Err(crate::error::NaadError::InvalidSampleRate { sample_rate });
        }
        if fundamental <= 0.0 || !fundamental.is_finite() {
            return Err(crate::error::NaadError::InvalidFrequency {
                frequency: fundamental,
                nyquist: sample_rate / 2.0,
            });
        }

        let count = num_partials.clamp(1, MAX_PARTIALS);
        let mut partials = Vec::with_capacity(count);
        let nyquist = sample_rate / 2.0;

        for i in 0..count {
            let ratio = (i + 1) as f32;
            // Skip partials above Nyquist.
            let amp = if fundamental * ratio < nyquist {
                1.0 / ratio
            } else {
                0.0
            };
            partials.push(Partial {
                frequency_ratio: ratio,
                amplitude: amp,
                phase: 0.0,
            });
        }

        let phases = vec![0.0; count];

        Ok(Self {
            fundamental,
            partials,
            phases,
            sample_rate,
        })
    }

    /// Set the fundamental frequency.
    ///
    /// # Errors
    ///
    /// Returns error if frequency is invalid.
    pub fn set_fundamental(&mut self, freq: f32) -> Result<()> {
        if freq <= 0.0 || !freq.is_finite() {
            return Err(crate::error::NaadError::InvalidFrequency {
                frequency: freq,
                nyquist: self.sample_rate / 2.0,
            });
        }
        self.fundamental = freq;
        Ok(())
    }

    /// Configure an individual partial by index.
    ///
    /// Does nothing if `index` is out of range.
    pub fn set_partial(&mut self, index: usize, freq_ratio: f32, amplitude: f32) {
        if let Some(p) = self.partials.get_mut(index) {
            p.frequency_ratio = freq_ratio;
            p.amplitude = amplitude.clamp(0.0, 1.0);
        }
    }

    /// Returns the number of partials.
    #[must_use]
    pub fn num_partials(&self) -> usize {
        self.partials.len()
    }

    /// Returns the fundamental frequency.
    #[must_use]
    pub fn fundamental(&self) -> f32 {
        self.fundamental
    }

    /// Generate the next output sample.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let mut sum = 0.0f32;

        for (i, partial) in self.partials.iter().enumerate() {
            if partial.amplitude <= 0.0 {
                continue;
            }
            let phase = self.phases[i];
            sum += (phase * std::f32::consts::TAU).sin() * partial.amplitude;

            let freq = self.fundamental * partial.frequency_ratio;
            let inc = freq / self.sample_rate;
            let new_phase = self.phases[i] + inc;
            self.phases[i] = new_phase - new_phase.floor();
        }

        sum
    }

    /// Fill a buffer with samples.
    #[inline]
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for s in buffer.iter_mut() {
            *s = self.next_sample();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_partial_is_sine() {
        let mut synth = AdditiveSynth::new(440.0, 1, 44100.0).unwrap();
        let mut buf = [0.0f32; 1024];
        synth.fill_buffer(&mut buf);

        // Should produce a clean sine wave.
        assert!(
            buf.iter().any(|&s| s.abs() > 0.5),
            "single partial should produce sine output"
        );
        assert!(buf.iter().all(|s| s.is_finite()));

        // Verify approximate sine shape: max value should be close to 1.0
        // (the first partial has amplitude 1.0).
        let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!((peak - 1.0).abs() < 0.05, "peak should be near 1.0");
    }

    #[test]
    fn test_harmonic_series() {
        let mut synth = AdditiveSynth::new(440.0, 8, 44100.0).unwrap();
        let mut buf = [0.0f32; 1024];
        synth.fill_buffer(&mut buf);
        assert!(buf.iter().any(|&s| s.abs() > 0.5));
        assert!(buf.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_set_partial() {
        let mut synth = AdditiveSynth::new(440.0, 4, 44100.0).unwrap();
        synth.set_partial(0, 1.0, 0.0);
        synth.set_partial(1, 2.0, 1.0);

        let mut buf = [0.0f32; 1024];
        synth.fill_buffer(&mut buf);
        // Should still produce output from partial 1 (at 2x fundamental).
        assert!(buf.iter().any(|&s| s.abs() > 0.5));
    }

    #[test]
    fn test_serde_roundtrip() {
        let synth = AdditiveSynth::new(440.0, 16, 44100.0).unwrap();
        let json = serde_json::to_string(&synth).unwrap();
        let back: AdditiveSynth = serde_json::from_str(&json).unwrap();
        assert_eq!(synth.partials.len(), back.partials.len());
        assert!((synth.fundamental - back.fundamental).abs() < f32::EPSILON);
    }

    #[test]
    fn test_nyquist_filtering() {
        // At 8000 Hz sample rate, partials above 4000 Hz should be silent.
        let synth = AdditiveSynth::new(1000.0, 8, 8000.0).unwrap();
        // Partials 5-8 (5000, 6000, 7000, 8000 Hz) should have zero amplitude.
        for i in 4..synth.partials.len() {
            assert!(
                synth.partials[i].amplitude == 0.0,
                "partial {} should be zeroed (above Nyquist)",
                i + 1
            );
        }
    }
}
