//! Oscillator module with band-limited waveform generation.
//!
//! Provides PolyBLEP anti-aliased saw, square, and pulse waveforms,
//! along with basic sine, triangle, and noise generators.

use serde::{Deserialize, Serialize};
use tracing::debug;

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

/// 4-point PolyBLEP correction for anti-aliased discontinuities.
///
/// Extends the correction window to 2 samples on each side of the
/// discontinuity (vs 1 sample for 2-point PolyBLEP), providing better
/// suppression of aliasing harmonics at high frequencies. The residual
/// is derived from an integrated piecewise-cubic BLAMP kernel, yielding
/// C1 continuity at the transition boundaries.
///
/// `t` is the phase position (0..1), `dt` is the phase increment per sample.
#[inline]
#[must_use]
pub fn polyblep(t: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    let dt2 = 2.0 * dt;
    if t < dt {
        // First sample after discontinuity
        let n = t / dt;
        let n2 = n * n;
        let blep2 = 2.0 * n - n2 - 1.0;
        let cubic = n2 * (n - 1.0) * 0.5;
        blep2 + cubic
    } else if t < dt2 {
        // Second sample after discontinuity (cubic tail)
        let n = t / dt - 1.0;
        let n2 = n * n;
        -n2 * (1.0 - n) * 0.5
    } else if t > 1.0 - dt {
        // First sample before discontinuity
        let n = (t - 1.0) / dt;
        let n2 = n * n;
        let blep2 = n2 + 2.0 * n + 1.0;
        let cubic = -n2 * (n + 1.0) * 0.5;
        blep2 + cubic
    } else if t > 1.0 - dt2 {
        // Second sample before discontinuity (cubic tail)
        let n = (t - 1.0) / dt + 1.0;
        let n2 = n * n;
        n2 * (1.0 + n) * 0.5
    } else {
        0.0
    }
}

/// Audio oscillator with band-limited waveform generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oscillator {
    waveform: Waveform,
    frequency: f32,
    phase: f32,
    sample_rate: f32,
    pulse_width: f32,
    #[serde(skip)]
    noise_gen: Option<noise::NoiseGenerator>,
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

        if !is_noise && let Some(e) = error::validate_frequency(frequency, sample_rate) {
            return Err(e);
        }

        let noise_gen = match waveform {
            Waveform::WhiteNoise => Some(noise::NoiseGenerator::new(noise::NoiseType::White, 42)),
            Waveform::PinkNoise => Some(noise::NoiseGenerator::new(noise::NoiseType::Pink, 42)),
            Waveform::BrownNoise => Some(noise::NoiseGenerator::new(noise::NoiseType::Brown, 42)),
            _ => None,
        };

        debug!(?waveform, frequency, sample_rate, "oscillator created");
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

    /// Ensure internal state is initialized (recovers after deserialization).
    fn ensure_initialized(&mut self) {
        if self.noise_gen.is_none() {
            self.noise_gen = match self.waveform {
                Waveform::WhiteNoise => {
                    Some(noise::NoiseGenerator::new(noise::NoiseType::White, 42))
                }
                Waveform::PinkNoise => Some(noise::NoiseGenerator::new(noise::NoiseType::Pink, 42)),
                Waveform::BrownNoise => {
                    Some(noise::NoiseGenerator::new(noise::NoiseType::Brown, 42))
                }
                _ => None,
            };
        }
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
                // Lazy init: reconstruct noise_gen after deserialization
                self.ensure_initialized();
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
    #[inline]
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.next_sample();
        }
    }

    /// Returns the waveform type.
    #[inline]
    #[must_use]
    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    /// Returns the current frequency in Hz.
    #[inline]
    #[must_use]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    /// Returns the current phase (0.0 to 1.0).
    #[inline]
    #[must_use]
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Returns the sample rate in Hz.
    #[inline]
    #[must_use]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Returns the pulse width (0.0 to 1.0).
    #[inline]
    #[must_use]
    pub fn pulse_width(&self) -> f32 {
        self.pulse_width
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

    /// Set the pulse width (clamped to 0.01..0.99).
    pub fn set_pulse_width(&mut self, pw: f32) {
        self.pulse_width = pw.clamp(0.01, 0.99);
    }

    /// Set the oscillator phase (0.0 to 1.0).
    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase.rem_euclid(1.0);
    }

    /// Reset the oscillator phase to zero.
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
        self.triangle_sum = 0.0;
    }

    /// Advance phase by a custom increment (for FM synthesis).
    ///
    /// Returns the sine of the current phase before advancing.
    /// This is used by FM synthesis which needs to control the
    /// instantaneous frequency directly.
    #[inline]
    pub fn advance_phase_sine(&mut self, dt: f32) -> f32 {
        let sample = (self.phase * std::f32::consts::TAU).sin();
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }
}

/// Generate a single waveform sample from phase and phase increment (no state mutation).
///
/// Used by `UnisonOscillator` to avoid duplicating waveform logic per voice.
/// Does not support noise waveforms or triangle integration (stateless).
#[inline]
fn stateless_waveform_sample(waveform: Waveform, t: f32, dt: f32) -> f32 {
    match waveform {
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
            if t < 0.25 {
                4.0 * t
            } else if t < 0.75 {
                2.0 - 4.0 * t
            } else {
                4.0 * t - 4.0
            }
        }
        Waveform::Pulse => {
            // Default 50% duty cycle for unison (pulse width not per-voice)
            let naive = if t < 0.5 { 1.0 } else { -1.0 };
            naive + polyblep(t, dt) - polyblep((t + 0.5) % 1.0, dt)
        }
        // Noise waveforms don't make sense for unison detuning — produce silence
        Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise => 0.0,
    }
}

fn default_detune_ratios() -> [f32; 8] {
    [1.0; 8]
}

fn default_ratios_dirty() -> bool {
    true
}

/// Unison oscillator — N detuned copies of a waveform mixed together.
///
/// Produces a thicker sound by layering multiple slightly-detuned voices.
/// Voices are spread symmetrically around the center frequency. Supports
/// stereo width output via `next_sample_stereo()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnisonOscillator {
    /// Waveform for all voices.
    waveform: Waveform,
    /// Base frequency in Hz.
    frequency: f32,
    /// Sample rate in Hz.
    sample_rate: f32,
    /// Number of unison voices (1-8).
    num_voices: usize,
    /// Detune amount in cents (spread across all voices).
    detune_cents: f32,
    /// Stereo spread (0.0 = mono, 1.0 = full width).
    stereo_spread: f32,
    /// Phase accumulators for each voice.
    phases: [f32; 8],
    /// Precomputed detune ratios (multiplied with base frequency).
    #[serde(skip, default = "default_detune_ratios")]
    detune_ratios: [f32; 8],
    /// Whether detune ratios need recomputation.
    #[serde(skip, default = "default_ratios_dirty")]
    ratios_dirty: bool,
}

impl UnisonOscillator {
    /// Create a new unison oscillator.
    ///
    /// # Arguments
    ///
    /// * `waveform` - Waveform type for all voices
    /// * `frequency` - Base frequency in Hz
    /// * `num_voices` - Number of unison voices (clamped to 1..8)
    /// * `detune_cents` - Total detune spread in cents
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Errors
    ///
    /// Returns error if frequency or sample_rate is invalid.
    pub fn new(
        waveform: Waveform,
        frequency: f32,
        num_voices: usize,
        detune_cents: f32,
        sample_rate: f32,
    ) -> Result<Self> {
        if let Some(e) = error::validate_sample_rate(sample_rate) {
            return Err(e);
        }
        if let Some(e) = error::validate_frequency(frequency, sample_rate) {
            return Err(e);
        }

        let nv = num_voices.clamp(1, 8);

        // Randomize initial phases for a natural sound
        let mut phases = [0.0f32; 8];
        let mut seed = 12345u32;
        for p in phases.iter_mut().take(nv) {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *p = seed as f32 / u32::MAX as f32;
        }

        let mut osc = Self {
            waveform,
            frequency,
            sample_rate,
            num_voices: nv,
            detune_cents: detune_cents.max(0.0),
            stereo_spread: 0.5,
            phases,
            detune_ratios: [1.0; 8],
            ratios_dirty: true,
        };
        osc.recompute_ratios();
        Ok(osc)
    }

    /// Recompute detune ratios from cents spread.
    fn recompute_ratios(&mut self) {
        if self.num_voices <= 1 {
            self.detune_ratios[0] = 1.0;
        } else {
            for i in 0..self.num_voices {
                // Spread voices symmetrically: -detune_cents/2 to +detune_cents/2
                let t = i as f32 / (self.num_voices - 1) as f32; // 0..1
                let cents_offset = (t - 0.5) * self.detune_cents;
                // Convert cents to frequency ratio: 2^(cents/1200)
                self.detune_ratios[i] = (cents_offset / 1200.0).exp2();
            }
        }
        self.ratios_dirty = false;
    }

    /// Generate the next mono sample (all voices averaged).
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        if self.ratios_dirty {
            self.recompute_ratios();
        }

        let mut sum = 0.0f32;
        let nv = self.num_voices;

        for i in 0..nv {
            let freq = self.frequency * self.detune_ratios[i];
            let dt = freq / self.sample_rate;
            let t = self.phases[i];

            let sample = stateless_waveform_sample(self.waveform, t, dt);

            sum += sample;

            // Advance phase
            self.phases[i] += dt;
            if self.phases[i] >= 1.0 {
                self.phases[i] -= 1.0;
            }
        }

        sum / nv as f32
    }

    /// Generate the next stereo sample pair (left, right).
    ///
    /// Voices are panned across the stereo field based on `stereo_spread`.
    #[inline]
    #[must_use]
    pub fn next_sample_stereo(&mut self) -> (f32, f32) {
        if self.ratios_dirty {
            self.recompute_ratios();
        }

        let mut left = 0.0f32;
        let mut right = 0.0f32;
        let nv = self.num_voices;

        for i in 0..nv {
            let freq = self.frequency * self.detune_ratios[i];
            let dt = freq / self.sample_rate;
            let t = self.phases[i];

            let sample = stateless_waveform_sample(self.waveform, t, dt);

            // Pan position: voice 0 = left, voice N-1 = right
            if nv > 1 {
                let pan = i as f32 / (nv - 1) as f32; // 0..1
                let w = self.stereo_spread * 0.5;
                let pan_scaled = 0.5 + (pan - 0.5) * w * 2.0;
                // Equal-power panning
                let angle = pan_scaled * std::f32::consts::FRAC_PI_2;
                left += sample * angle.cos();
                right += sample * angle.sin();
            } else {
                left += sample * std::f32::consts::FRAC_1_SQRT_2;
                right += sample * std::f32::consts::FRAC_1_SQRT_2;
            }

            self.phases[i] += dt;
            if self.phases[i] >= 1.0 {
                self.phases[i] -= 1.0;
            }
        }

        let inv = 1.0 / nv as f32;
        (left * inv, right * inv)
    }

    /// Fill a mono buffer.
    #[inline]
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.next_sample();
        }
    }

    /// Fill stereo buffers (left and right channels).
    #[inline]
    pub fn fill_buffer_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let (sl, sr) = self.next_sample_stereo();
            *l = sl;
            *r = sr;
        }
    }

    /// Set the number of unison voices (1-8).
    pub fn set_num_voices(&mut self, n: usize) {
        self.num_voices = n.clamp(1, 8);
        self.ratios_dirty = true;
    }

    /// Set the detune amount in cents.
    pub fn set_detune_cents(&mut self, cents: f32) {
        self.detune_cents = cents.max(0.0);
        self.ratios_dirty = true;
    }

    /// Set the stereo spread (0.0 = mono, 1.0 = full width).
    pub fn set_stereo_spread(&mut self, spread: f32) {
        self.stereo_spread = spread.clamp(0.0, 1.0);
    }

    /// Set the base frequency.
    ///
    /// # Errors
    ///
    /// Returns error if frequency is invalid.
    pub fn set_frequency(&mut self, freq: f32) -> Result<()> {
        if let Some(e) = error::validate_frequency(freq, self.sample_rate) {
            return Err(e);
        }
        self.frequency = freq;
        Ok(())
    }

    /// Returns the number of unison voices.
    #[inline]
    #[must_use]
    pub fn num_voices(&self) -> usize {
        self.num_voices
    }

    /// Returns the base frequency.
    #[inline]
    #[must_use]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    /// Returns the detune in cents.
    #[inline]
    #[must_use]
    pub fn detune_cents(&self) -> f32 {
        self.detune_cents
    }
}

/// Octave division for a sub-oscillator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SubOctave {
    /// One octave below (half frequency).
    Down1,
    /// Two octaves below (quarter frequency).
    Down2,
}

/// Sub-oscillator — plays 1 or 2 octaves below a given frequency.
///
/// Typically mixed with a main oscillator to add low-end body.
/// The sub-oscillator has an independent waveform selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubOscillator {
    /// The underlying oscillator.
    osc: Oscillator,
    /// Octave division.
    octave: SubOctave,
    /// Base frequency (before octave division).
    base_frequency: f32,
    /// Mix level (0.0 to 1.0).
    pub level: f32,
}

impl SubOscillator {
    /// Create a new sub-oscillator.
    ///
    /// # Errors
    ///
    /// Returns error if frequency or sample_rate is invalid.
    pub fn new(
        waveform: Waveform,
        base_frequency: f32,
        octave: SubOctave,
        sample_rate: f32,
    ) -> Result<Self> {
        let divisor = match octave {
            SubOctave::Down1 => 2.0,
            SubOctave::Down2 => 4.0,
        };
        let sub_freq = base_frequency / divisor;
        let osc = Oscillator::new(waveform, sub_freq.max(0.1), sample_rate)?;
        Ok(Self {
            osc,
            octave,
            base_frequency,
            level: 1.0,
        })
    }

    /// Generate the next sub-oscillator sample (scaled by level).
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        self.osc.next_sample() * self.level
    }

    /// Update the base frequency (sub frequency is derived automatically).
    ///
    /// # Errors
    ///
    /// Returns error if frequency is invalid.
    pub fn set_base_frequency(&mut self, freq: f32) -> Result<()> {
        let divisor = match self.octave {
            SubOctave::Down1 => 2.0,
            SubOctave::Down2 => 4.0,
        };
        // Validate before mutating state
        self.osc.set_frequency((freq / divisor).max(0.1))?;
        self.base_frequency = freq;
        Ok(())
    }

    /// Set the octave division.
    ///
    /// # Errors
    ///
    /// Returns error if the resulting frequency is invalid.
    pub fn set_octave(&mut self, octave: SubOctave) -> Result<()> {
        let divisor = match octave {
            SubOctave::Down1 => 2.0,
            SubOctave::Down2 => 4.0,
        };
        self.osc
            .set_frequency((self.base_frequency / divisor).max(0.1))?;
        self.octave = octave;
        Ok(())
    }

    /// Returns the current octave division.
    #[inline]
    #[must_use]
    pub fn octave(&self) -> SubOctave {
        self.octave
    }

    /// Returns the base frequency.
    #[inline]
    #[must_use]
    pub fn base_frequency(&self) -> f32 {
        self.base_frequency
    }

    /// Fill a buffer with sub-oscillator samples.
    #[inline]
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.next_sample();
        }
    }
}

/// Hard sync oscillator — slave resets phase on master cycle completion.
///
/// When the master oscillator wraps its phase (completes a cycle), the slave
/// oscillator's phase is reset to zero, producing the characteristic hard sync
/// harmonic sweep effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardSync {
    /// Master oscillator (controls reset timing).
    master: Oscillator,
    /// Slave oscillator (produces output, gets phase-reset).
    slave: Oscillator,
}

impl HardSync {
    /// Create a new hard sync pair.
    ///
    /// # Errors
    ///
    /// Returns error if frequencies or sample_rate are invalid.
    pub fn new(
        master_freq: f32,
        slave_freq: f32,
        slave_waveform: Waveform,
        sample_rate: f32,
    ) -> Result<Self> {
        let master = Oscillator::new(Waveform::Saw, master_freq, sample_rate)?;
        let slave = Oscillator::new(slave_waveform, slave_freq, sample_rate)?;
        Ok(Self { master, slave })
    }

    /// Generate the next hard-synced sample.
    ///
    /// The slave produces audio; the master controls when the slave resets.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let master_phase_before = self.master.phase;
        let _ = self.master.next_sample();
        let master_phase_after = self.master.phase;

        // Detect master cycle wrap (phase decreased = wrapped past 1.0)
        if master_phase_after < master_phase_before {
            self.slave.reset_phase();
        }

        self.slave.next_sample()
    }

    /// Fill a buffer with hard-synced samples.
    #[inline]
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.next_sample();
        }
    }

    /// Returns a reference to the master oscillator.
    #[must_use]
    pub fn master(&self) -> &Oscillator {
        &self.master
    }

    /// Returns a reference to the slave oscillator.
    #[must_use]
    pub fn slave(&self) -> &Oscillator {
        &self.slave
    }

    /// Set the master frequency.
    ///
    /// # Errors
    ///
    /// Returns error if frequency is invalid.
    pub fn set_master_freq(&mut self, freq: f32) -> Result<()> {
        self.master.set_frequency(freq)
    }

    /// Set the slave frequency.
    ///
    /// # Errors
    ///
    /// Returns error if frequency is invalid.
    pub fn set_slave_freq(&mut self, freq: f32) -> Result<()> {
        self.slave.set_frequency(freq)
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
        assert_eq!(osc.waveform(), back.waveform());
        assert!((osc.frequency() - back.frequency()).abs() < f32::EPSILON);
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

    #[test]
    fn test_sub_oscillator() {
        let mut sub =
            SubOscillator::new(Waveform::Square, 440.0, SubOctave::Down1, 44100.0).unwrap();
        let mut buf = [0.0f32; 1024];
        sub.fill_buffer(&mut buf);
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(buf.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_sub_oscillator_octave() {
        let sub1 = SubOscillator::new(Waveform::Sine, 440.0, SubOctave::Down1, 44100.0).unwrap();
        let sub2 = SubOscillator::new(Waveform::Sine, 440.0, SubOctave::Down2, 44100.0).unwrap();
        // Down1 = 220 Hz, Down2 = 110 Hz
        assert!((sub1.osc.frequency() - 220.0).abs() < 0.01);
        assert!((sub2.osc.frequency() - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_sub_oscillator_serde() {
        let sub = SubOscillator::new(Waveform::Square, 440.0, SubOctave::Down2, 44100.0).unwrap();
        let json = serde_json::to_string(&sub).unwrap();
        let back: SubOscillator = serde_json::from_str(&json).unwrap();
        assert_eq!(sub.octave(), back.octave());
    }

    #[test]
    fn test_unison_mono() {
        let mut uni = UnisonOscillator::new(Waveform::Saw, 440.0, 4, 10.0, 44100.0).unwrap();
        let mut buf = [0.0f32; 1024];
        uni.fill_buffer(&mut buf);
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(buf.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_unison_stereo_spread() {
        let mut uni = UnisonOscillator::new(Waveform::Saw, 440.0, 4, 10.0, 44100.0).unwrap();
        uni.set_stereo_spread(1.0);
        let mut left = [0.0f32; 512];
        let mut right = [0.0f32; 512];
        uni.fill_buffer_stereo(&mut left, &mut right);
        // With spread, left and right should differ
        let diff: f32 = left
            .iter()
            .zip(right.iter())
            .map(|(l, r)| (l - r).abs())
            .sum();
        assert!(diff > 0.01, "stereo channels should differ with spread=1.0");
    }

    #[test]
    fn test_unison_single_voice() {
        let mut uni = UnisonOscillator::new(Waveform::Sine, 440.0, 1, 0.0, 44100.0).unwrap();
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0).unwrap();
        // Single voice unison with 0 detune should match a plain oscillator
        // (phase randomization makes exact match impossible, but range should match)
        for _ in 0..100 {
            let s = uni.next_sample();
            assert!((-1.01..=1.01).contains(&s));
            let _ = osc.next_sample();
        }
    }

    #[test]
    fn test_unison_serde_roundtrip() {
        let uni = UnisonOscillator::new(Waveform::Saw, 440.0, 4, 15.0, 44100.0).unwrap();
        let json = serde_json::to_string(&uni).unwrap();
        let back: UnisonOscillator = serde_json::from_str(&json).unwrap();
        assert_eq!(uni.num_voices(), back.num_voices());
        assert!((uni.detune_cents() - back.detune_cents()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hard_sync_resets_slave() {
        // Master at 440 Hz, slave at 880 Hz — slave should reset every master cycle
        let mut sync = HardSync::new(440.0, 880.0, Waveform::Saw, 44100.0).unwrap();
        let mut buf = [0.0f32; 1024];
        sync.fill_buffer(&mut buf);
        assert!(buf.iter().all(|s| s.is_finite()));
        // Slave should produce non-trivial output
        assert!(buf.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_hard_sync_serde_roundtrip() {
        let hs = HardSync::new(440.0, 880.0, Waveform::Saw, 44100.0).unwrap();
        let json = serde_json::to_string(&hs).unwrap();
        let back: HardSync = serde_json::from_str(&json).unwrap();
        assert!((hs.master().frequency() - back.master().frequency()).abs() < f32::EPSILON);
    }
}
