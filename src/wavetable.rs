//! Wavetable synthesis with morphing support.
//!
//! Provides wavetable oscillators with linear interpolation and
//! the ability to morph between multiple wavetables.

use serde::{Deserialize, Serialize};

use crate::error::{self, NaadError, Result};

/// A single wavetable containing one cycle of a waveform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wavetable {
    /// The waveform samples (one cycle).
    pub samples: Vec<f32>,
}

impl Wavetable {
    /// Create a wavetable from raw samples.
    ///
    /// # Errors
    ///
    /// Returns `NaadError::InvalidParameter` if samples is empty.
    pub fn from_samples(samples: Vec<f32>) -> Result<Self> {
        if samples.is_empty() {
            return Err(NaadError::InvalidParameter {
                name: "samples".to_string(),
                reason: "wavetable must have at least one sample".to_string(),
            });
        }
        Ok(Self { samples })
    }

    /// Create a wavetable from additive harmonics.
    ///
    /// Generates a wavetable of `size` samples by summing sine waves at
    /// integer multiples of the fundamental, weighted by `amplitudes`.
    ///
    /// # Errors
    ///
    /// Returns `NaadError::InvalidParameter` if `num_harmonics` is 0,
    /// `amplitudes` is empty, or `size` is 0.
    pub fn from_harmonics(num_harmonics: usize, amplitudes: &[f32], size: usize) -> Result<Self> {
        if num_harmonics == 0 {
            return Err(NaadError::InvalidParameter {
                name: "num_harmonics".to_string(),
                reason: "must be > 0".to_string(),
            });
        }
        if amplitudes.is_empty() {
            return Err(NaadError::InvalidParameter {
                name: "amplitudes".to_string(),
                reason: "must not be empty".to_string(),
            });
        }
        if size == 0 {
            return Err(NaadError::InvalidParameter {
                name: "size".to_string(),
                reason: "must be > 0".to_string(),
            });
        }

        let mut samples = vec![0.0f32; size];
        let harmonics_to_generate = num_harmonics.min(amplitudes.len());

        for (h, &amp) in amplitudes.iter().take(harmonics_to_generate).enumerate() {
            let harmonic_num = (h + 1) as f32;
            for (i, sample) in samples.iter_mut().enumerate() {
                let phase = (i as f32 / size as f32) * std::f32::consts::TAU * harmonic_num;
                *sample += amp * phase.sin();
            }
        }

        // Normalize to -1..1
        let max_abs = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if max_abs > 0.0 {
            for sample in &mut samples {
                *sample /= max_abs;
            }
        }

        Ok(Self { samples })
    }

    /// Read a sample from the wavetable with linear interpolation.
    #[inline]
    #[must_use]
    pub fn read_interpolated(&self, phase: f32) -> f32 {
        let len = self.samples.len() as f32;
        let index = phase * len;
        let index_floor = index.floor();
        let frac = index - index_floor;

        let i0 = (index_floor as usize) % self.samples.len();
        let i1 = (i0 + 1) % self.samples.len();

        self.samples[i0] * (1.0 - frac) + self.samples[i1] * frac
    }
}

/// Wavetable oscillator that reads from a single wavetable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WavetableOscillator {
    /// The wavetable to read from.
    pub table: Wavetable,
    /// Current phase (0.0 to 1.0).
    pub phase: f32,
    /// Sample rate in Hz.
    pub sample_rate: f32,
    /// Playback frequency in Hz.
    pub frequency: f32,
}

impl WavetableOscillator {
    /// Create a new wavetable oscillator.
    ///
    /// # Errors
    ///
    /// Returns error if sample_rate or frequency is invalid.
    pub fn new(table: Wavetable, frequency: f32, sample_rate: f32) -> Result<Self> {
        if let Some(e) = error::validate_sample_rate(sample_rate) {
            return Err(e);
        }
        if let Some(e) = error::validate_frequency(frequency, sample_rate) {
            return Err(e);
        }
        Ok(Self {
            table,
            phase: 0.0,
            sample_rate,
            frequency,
        })
    }

    /// Generate the next sample with linear interpolation.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let sample = self.table.read_interpolated(self.phase);

        self.phase += self.frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }

    /// Fill a buffer with wavetable samples.
    #[inline]
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.next_sample();
        }
    }
}

/// A collection of wavetables that can be morphed between.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphWavetable {
    /// The wavetables to morph between.
    pub tables: Vec<Wavetable>,
    /// Morph position (0.0 to 1.0).
    pub position: f32,
    /// Current phase (0.0 to 1.0).
    pub phase: f32,
    /// Sample rate in Hz.
    pub sample_rate: f32,
    /// Playback frequency in Hz.
    pub frequency: f32,
}

impl MorphWavetable {
    /// Create a new morph wavetable.
    ///
    /// # Errors
    ///
    /// Returns error if tables is empty, or sample_rate/frequency is invalid.
    pub fn new(tables: Vec<Wavetable>, frequency: f32, sample_rate: f32) -> Result<Self> {
        if tables.is_empty() {
            return Err(NaadError::InvalidParameter {
                name: "tables".to_string(),
                reason: "must have at least one wavetable".to_string(),
            });
        }
        // Validate all tables have the same size for correct morphing
        let first_len = tables[0].samples.len();
        if tables.iter().any(|t| t.samples.len() != first_len) {
            return Err(NaadError::InvalidParameter {
                name: "tables".to_string(),
                reason: "all wavetables must have the same number of samples".to_string(),
            });
        }
        if let Some(e) = error::validate_sample_rate(sample_rate) {
            return Err(e);
        }
        if let Some(e) = error::validate_frequency(frequency, sample_rate) {
            return Err(e);
        }
        Ok(Self {
            tables,
            position: 0.0,
            phase: 0.0,
            sample_rate,
            frequency,
        })
    }

    /// Set the morph position (clamped to 0.0..1.0).
    pub fn set_morph(&mut self, position: f32) {
        self.position = position.clamp(0.0, 1.0);
    }

    /// Generate the next sample, interpolating between wavetables.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let num_tables = self.tables.len();
        let sample = if num_tables == 1 {
            self.tables[0].read_interpolated(self.phase)
        } else {
            let scaled = self.position * (num_tables - 1) as f32;
            let idx_low = (scaled.floor() as usize).min(num_tables - 2);
            let idx_high = idx_low + 1;
            let frac = scaled - idx_low as f32;

            let s_low = self.tables[idx_low].read_interpolated(self.phase);
            let s_high = self.tables[idx_high].read_interpolated(self.phase);
            s_low * (1.0 - frac) + s_high * frac
        };

        self.phase += self.frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_samples() {
        let wt = Wavetable::from_samples(vec![0.0, 1.0, 0.0, -1.0]).unwrap();
        assert_eq!(wt.samples.len(), 4);
    }

    #[test]
    fn test_from_samples_empty() {
        assert!(Wavetable::from_samples(vec![]).is_err());
    }

    #[test]
    fn test_from_harmonics() {
        let wt = Wavetable::from_harmonics(3, &[1.0, 0.5, 0.25], 1024).unwrap();
        assert_eq!(wt.samples.len(), 1024);
        let max = wt.samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!((max - 1.0).abs() < 0.01, "should be normalized to 1.0");
    }

    #[test]
    fn test_interpolated_read() {
        let wt = Wavetable::from_samples(vec![0.0, 1.0, 0.0, -1.0]).unwrap();
        let s = wt.read_interpolated(0.125); // between index 0 and 1
        assert!(s > 0.0 && s < 1.0);
    }

    #[test]
    fn test_wavetable_oscillator() {
        let wt = Wavetable::from_harmonics(1, &[1.0], 256).unwrap();
        let mut osc = WavetableOscillator::new(wt, 440.0, 44100.0).unwrap();
        let mut buf = [0.0f32; 256];
        osc.fill_buffer(&mut buf);
        assert!(buf.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_morph_wavetable() {
        let wt1 = Wavetable::from_harmonics(1, &[1.0], 256).unwrap();
        let wt2 = Wavetable::from_harmonics(2, &[1.0, 0.5], 256).unwrap();
        let mut morph = MorphWavetable::new(vec![wt1, wt2], 440.0, 44100.0).unwrap();
        morph.set_morph(0.5);
        let s = morph.next_sample();
        assert!(s.is_finite());
    }

    #[test]
    fn test_serde_roundtrip() {
        let wt = Wavetable::from_samples(vec![0.0, 1.0, 0.0, -1.0]).unwrap();
        let json = serde_json::to_string(&wt).unwrap();
        let back: Wavetable = serde_json::from_str(&json).unwrap();
        assert_eq!(wt.samples, back.samples);
    }
}
