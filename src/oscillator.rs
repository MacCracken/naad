//! Oscillator module with band-limited waveform generation.
//!
//! Provides PolyBLEP anti-aliased saw, square, and pulse waveforms,
//! along with basic sine, triangle, and noise generators.

use serde::{Deserialize, Serialize};

use crate::error::{self, Result};
use crate::noise;

/// Waveform type for an oscillator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Waveform {
    /// Sine wave.
    Sine,
    /// Band-limited sawtooth wave (PolyBLEP).
    Saw,
    /// Band-limited square wave (PolyBLEP).
    Square,
    /// Triangle wave (integrated square).
    Triangle,
    /// Band-limited pulse wave with variable width (PolyBLEP).
    Pulse,
    /// White noise.
    WhiteNoise,
    /// Pink noise (Voss-McCartney).
    PinkNoise,
    /// Brown noise (integrated white).
    BrownNoise,
}

/// PolyBLEP correction for anti-aliased discontinuities.
///
/// `t` is the phase position (0..1), `dt` is the phase increment per sample.
#[inline]
#[must_use]
pub fn polyblep(t: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    if t < dt {
        let t = t / dt;
        2.0 * t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + 2.0 * t + 1.0
    } else {
        0.0
    }
}

/// Audio oscillator with band-limited waveform generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oscillator {
    /// The waveform type.
    pub waveform: Waveform,
    /// Frequency in Hz.
    pub frequency: f32,
    /// Current phase (0.0 to 1.0).
    pub phase: f32,
    /// Sample rate in Hz.
    pub sample_rate: f32,
    /// Pulse width for Pulse waveform (0.0 to 1.0, default 0.5).
    pub pulse_width: f32,
    /// Noise generator state for noise waveforms.
    #[serde(skip)]
    noise_gen: Option<noise::NoiseGenerator>,
    /// Triangle integrator state.
    #[serde(skip)]
    triangle_sum: f32,
}

impl Oscillator {
    /// Create a new oscillator.
    ///
    /// # Errors
    ///
    /// Returns `NaadError::InvalidSampleRate` if sample_rate <= 0.
    /// Returns `NaadError::InvalidFrequency` if frequency is out of range
    /// (does not apply to noise waveforms).
    pub fn new(waveform: Waveform, frequency: f32, sample_rate: f32) -> Result<Self> {
        if let Some(e) = error::validate_sample_rate(sample_rate) {
            return Err(e);
        }

        let is_noise = matches!(
            waveform,
            Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise
        );

        if !is_noise {
            if let Some(e) = error::validate_frequency(frequency, sample_rate) {
                return Err(e);
            }
        }

        let noise_gen = match waveform {
            Waveform::WhiteNoise => Some(noise::NoiseGenerator::new(noise::NoiseType::White, 42)),
            Waveform::PinkNoise => Some(noise::NoiseGenerator::new(noise::NoiseType::Pink, 42)),
            Waveform::BrownNoise => Some(noise::NoiseGenerator::new(noise::NoiseType::Brown, 42)),
            _ => None,
        };

        Ok(Self {
            waveform,
            frequency,
            phase: 0.0,
            sample_rate,
            pulse_width: 0.5,
            noise_gen,
            triangle_sum: 0.0,
        })
    }

    /// Phase increment per sample.
    #[inline]
    #[must_use]
    pub fn phase_increment(&self) -> f32 {
        self.frequency / self.sample_rate
    }

    /// Generate the next sample.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let dt = self.phase_increment();
        let t = self.phase;

        let sample = match self.waveform {
            Waveform::Sine => (t * std::f32::consts::TAU).sin(),

            Waveform::Saw => {
                let naive = 2.0 * t - 1.0;
                naive - polyblep(t, dt)
            }

            Waveform::Square => {
                let naive = if t < 0.5 { 1.0 } else { -1.0 };
                naive + polyblep(t, dt) - polyblep((t + 0.5) % 1.0, dt)
            }

            Waveform::Triangle => {
                // Integrated square wave for triangle
                let square = if t < 0.5 { 1.0 } else { -1.0 };
                let square_blep = square + polyblep(t, dt) - polyblep((t + 0.5) % 1.0, dt);
                // Leaky integrator
                self.triangle_sum = 0.999 * self.triangle_sum + square_blep * dt * 4.0;
                self.triangle_sum.clamp(-1.0, 1.0)
            }

            Waveform::Pulse => {
                let pw = self.pulse_width.clamp(0.01, 0.99);
                let naive = if t < pw { 1.0 } else { -1.0 };
                naive + polyblep(t, dt) - polyblep((t + (1.0 - pw)) % 1.0, dt)
            }

            Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise => {
                if let Some(ref mut ng) = self.noise_gen {
                    ng.next_sample()
                } else {
                    0.0
                }
            }
        };

        // Advance phase
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }

    /// Fill a buffer with generated samples.
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.next_sample();
        }
    }

    /// Set the oscillator frequency.
    ///
    /// # Errors
    ///
    /// Returns `NaadError::InvalidFrequency` if frequency is out of valid range.
    pub fn set_frequency(&mut self, freq: f32) -> Result<()> {
        if let Some(e) = error::validate_frequency(freq, self.sample_rate) {
            return Err(e);
        }
        self.frequency = freq;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_range() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0).unwrap();
        for _ in 0..1000 {
            let s = osc.next_sample();
            assert!((-1.0..=1.0).contains(&s), "sample out of range: {s}");
        }
    }

    #[test]
    fn test_saw_range() {
        let mut osc = Oscillator::new(Waveform::Saw, 440.0, 44100.0).unwrap();
        for _ in 0..1000 {
            let s = osc.next_sample();
            assert!((-1.5..=1.5).contains(&s), "saw sample out of range: {s}");
        }
    }

    #[test]
    fn test_invalid_frequency() {
        assert!(Oscillator::new(Waveform::Sine, -1.0, 44100.0).is_err());
        assert!(Oscillator::new(Waveform::Sine, 0.0, 44100.0).is_err());
        assert!(Oscillator::new(Waveform::Sine, 25000.0, 44100.0).is_err());
    }

    #[test]
    fn test_invalid_sample_rate() {
        assert!(Oscillator::new(Waveform::Sine, 440.0, 0.0).is_err());
        assert!(Oscillator::new(Waveform::Sine, 440.0, -1.0).is_err());
    }

    #[test]
    fn test_set_frequency() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0).unwrap();
        assert!(osc.set_frequency(880.0).is_ok());
        assert!(osc.set_frequency(0.0).is_err());
    }

    #[test]
    fn test_fill_buffer() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0).unwrap();
        let mut buf = [0.0f32; 128];
        osc.fill_buffer(&mut buf);
        assert!(buf.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_serde_roundtrip() {
        let osc = Oscillator::new(Waveform::Saw, 440.0, 44100.0).unwrap();
        let json = serde_json::to_string(&osc).unwrap();
        let back: Oscillator = serde_json::from_str(&json).unwrap();
        assert_eq!(osc.waveform, back.waveform);
        assert!((osc.frequency - back.frequency).abs() < f32::EPSILON);
    }

    #[test]
    fn test_polyblep_function() {
        assert!((polyblep(0.5, 0.01) - 0.0).abs() < f32::EPSILON);
        assert!(polyblep(0.001, 0.01).abs() > 0.0);
    }

    #[test]
    fn test_noise_waveforms() {
        let mut osc = Oscillator::new(Waveform::WhiteNoise, 0.1, 44100.0).unwrap();
        let s = osc.next_sample();
        assert!(s.is_finite());
    }
}
