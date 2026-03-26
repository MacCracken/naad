//! Modulation sources: LFO, FM synthesis, and ring modulation.

use serde::{Deserialize, Serialize};

use crate::error::{self, Result};
use crate::oscillator::{Oscillator, Waveform};

/// Low-frequency oscillator for modulation.
///
/// Wraps an `Oscillator` — typically used below 20 Hz but not enforced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lfo {
    /// The underlying oscillator.
    osc: Oscillator,
    /// Modulation depth (amplitude scaling, 0.0 to 1.0).
    pub depth: f32,
}

impl Lfo {
    /// Create a new LFO.
    ///
    /// # Errors
    ///
    /// Returns error if frequency or sample_rate is invalid.
    pub fn new(waveform: Waveform, frequency: f32, sample_rate: f32) -> Result<Self> {
        let osc = Oscillator::new(waveform, frequency, sample_rate)?;
        Ok(Self { osc, depth: 1.0 })
    }

    /// Generate the next modulation value (scaled by depth).
    #[inline]
    pub fn next_value(&mut self) -> f32 {
        self.osc.next_sample() * self.depth
    }

    /// Set the LFO frequency.
    ///
    /// # Errors
    ///
    /// Returns error if frequency is invalid.
    pub fn set_frequency(&mut self, freq: f32) -> Result<()> {
        self.osc.set_frequency(freq)
    }
}

/// Trait for modulation sources.
pub trait ModulationSource {
    /// Generate the next modulation value.
    fn next_modulation_value(&mut self) -> f32;
}

impl ModulationSource for Lfo {
    fn next_modulation_value(&mut self) -> f32 {
        self.next_value()
    }
}

/// FM (Frequency Modulation) synthesizer.
///
/// The modulator output is scaled by `mod_index` and added to the carrier
/// frequency to produce frequency modulation and sidebands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FmSynth {
    /// Carrier oscillator.
    pub carrier: Oscillator,
    /// Modulator oscillator.
    pub modulator: Oscillator,
    /// Modulation index (depth of FM).
    pub mod_index: f32,
    /// Base carrier frequency for FM calculation.
    carrier_base_freq: f32,
}

impl FmSynth {
    /// Create a new FM synthesizer.
    ///
    /// # Arguments
    ///
    /// * `carrier_freq` - Carrier frequency in Hz
    /// * `mod_freq` - Modulator frequency in Hz
    /// * `mod_index` - Modulation index (ratio of frequency deviation to modulator frequency)
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Errors
    ///
    /// Returns error if frequencies or sample_rate are invalid.
    pub fn new(
        carrier_freq: f32,
        mod_freq: f32,
        mod_index: f32,
        sample_rate: f32,
    ) -> Result<Self> {
        if let Some(e) = error::validate_sample_rate(sample_rate) {
            return Err(e);
        }
        let carrier = Oscillator::new(Waveform::Sine, carrier_freq, sample_rate)?;
        let modulator = Oscillator::new(Waveform::Sine, mod_freq, sample_rate)?;
        Ok(Self {
            carrier,
            modulator,
            mod_index,
            carrier_base_freq: carrier_freq,
        })
    }

    /// Generate the next FM synthesis sample.
    ///
    /// Applies frequency modulation: carrier frequency is modulated by
    /// the modulator output scaled by mod_index * mod_frequency.
    #[inline]
    pub fn fm_next_sample(&mut self) -> f32 {
        // Get modulator output
        let mod_out = self.modulator.next_sample();

        // Calculate instantaneous carrier frequency
        let freq_deviation = mod_out * self.mod_index * self.modulator.frequency;
        let inst_freq = self.carrier_base_freq + freq_deviation;

        // Clamp to valid range
        let nyquist = self.carrier.sample_rate / 2.0;
        let clamped_freq = inst_freq.clamp(0.1, nyquist - 1.0);

        // Directly modulate the carrier's phase increment
        let dt = clamped_freq / self.carrier.sample_rate;
        let sample = (self.carrier.phase * std::f32::consts::TAU).sin();

        self.carrier.phase += dt;
        if self.carrier.phase >= 1.0 {
            self.carrier.phase -= 1.0;
        }

        sample
    }

    /// Fill a buffer with FM synthesis samples.
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.fm_next_sample();
        }
    }
}

/// Ring modulator — multiplies two signals together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingModulator {
    /// The modulator oscillator.
    pub modulator: Oscillator,
}

impl RingModulator {
    /// Create a new ring modulator.
    ///
    /// # Errors
    ///
    /// Returns error if frequency or sample_rate is invalid.
    pub fn new(waveform: Waveform, mod_freq: f32, sample_rate: f32) -> Result<Self> {
        let modulator = Oscillator::new(waveform, mod_freq, sample_rate)?;
        Ok(Self { modulator })
    }

    /// Process a sample through ring modulation.
    ///
    /// Multiplies the input by the modulator output.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        input * self.modulator.next_sample()
    }

    /// Process a buffer in place.
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfo_basic() {
        let mut lfo = Lfo::new(Waveform::Sine, 5.0, 44100.0).unwrap();
        let val = lfo.next_value();
        assert!(val.is_finite());
    }

    #[test]
    fn test_lfo_depth() {
        let mut lfo = Lfo::new(Waveform::Sine, 5.0, 44100.0).unwrap();
        lfo.depth = 0.5;
        for _ in 0..10000 {
            let val = lfo.next_value();
            assert!(
                val.abs() <= 0.51,
                "LFO with depth 0.5 should stay within bounds, got {val}"
            );
        }
    }

    #[test]
    fn test_fm_synthesis() {
        let mut fm = FmSynth::new(440.0, 220.0, 2.0, 44100.0).unwrap();
        let mut buf = [0.0f32; 1024];
        fm.fill_buffer(&mut buf);
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(buf.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_ring_modulator() {
        let mut ring = RingModulator::new(Waveform::Sine, 300.0, 44100.0).unwrap();
        let output = ring.process_sample(1.0);
        assert!(output.is_finite());
    }

    #[test]
    fn test_modulation_source_trait() {
        let mut lfo = Lfo::new(Waveform::Sine, 5.0, 44100.0).unwrap();
        let val = lfo.next_modulation_value();
        assert!(val.is_finite());
    }

    #[test]
    fn test_serde_roundtrip() {
        let fm = FmSynth::new(440.0, 220.0, 2.0, 44100.0).unwrap();
        let json = serde_json::to_string(&fm).unwrap();
        let back: FmSynth = serde_json::from_str(&json).unwrap();
        assert!((fm.mod_index - back.mod_index).abs() < f32::EPSILON);
    }
}
