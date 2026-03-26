//! Granular synthesis engine.
//!
//! Produces sound by overlapping many short "grains" — windowed
//! fragments read from a source buffer at controllable positions,
//! rates, and densities. Supports time-stretching, pitch-shifting,
//! and spectral smearing via the `spray` (time jitter) parameter.

use serde::{Deserialize, Serialize};

/// Maximum number of simultaneous grains.
const MAX_GRAINS: usize = 64;

/// Window function applied to each grain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GrainWindow {
    /// Hann (raised cosine) window — smooth, general purpose.
    Hann,
    /// Gaussian window — smooth with narrower main lobe.
    Gaussian,
    /// Tukey window (cosine-tapered rectangle).
    Tukey,
    /// Rectangular window (no tapering).
    Rectangular,
}

impl GrainWindow {
    /// Compute the window value at position `t` (0.0 to 1.0).
    #[inline]
    #[must_use]
    pub fn value(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            GrainWindow::Hann => 0.5 * (1.0 - (std::f32::consts::TAU * t).cos()),
            GrainWindow::Gaussian => {
                // sigma = 0.4, centred at 0.5.
                let x = (t - 0.5) / 0.4;
                (-0.5 * x * x).exp()
            }
            GrainWindow::Tukey => {
                // Tukey window with alpha = 0.5 (half cosine-tapered).
                let alpha = 0.5;
                let half_alpha = alpha * 0.5;
                if t < half_alpha {
                    // Leading taper: 0 → 1
                    0.5 * (1.0 - (std::f32::consts::PI * t / half_alpha).cos())
                } else if t > 1.0 - half_alpha {
                    // Trailing taper: 1 → 0
                    0.5 * (1.0 + (std::f32::consts::PI * (t - 1.0 + half_alpha) / half_alpha).cos())
                } else {
                    1.0
                }
            }
            GrainWindow::Rectangular => 1.0,
        }
    }
}

/// A single grain: a windowed fragment of the source buffer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Grain {
    /// Starting position in the source buffer (fractional sample).
    source_position: f32,
    /// Playback rate (1.0 = normal, 2.0 = double speed / octave up).
    playback_rate: f32,
    /// Window function for this grain.
    window: GrainWindow,
    /// Duration in samples.
    duration_samples: u32,
    /// Current sample within the grain.
    current_sample: u32,
    /// Amplitude scaling.
    amplitude: f32,
    /// Whether this grain slot is active.
    active: bool,
}

impl Default for Grain {
    fn default() -> Self {
        Self {
            source_position: 0.0,
            playback_rate: 1.0,
            window: GrainWindow::Hann,
            duration_samples: 0,
            current_sample: 0,
            amplitude: 1.0,
            active: false,
        }
    }
}

/// Granular synthesis engine.
///
/// Reads grains from a source buffer, applying windowing, pitch
/// shifting (via playback rate), time jitter (spray), and
/// configurable grain density.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GranularEngine {
    /// Source audio buffer.
    #[serde(skip)]
    source: Vec<f32>,
    /// Grain pool (fixed size).
    grains: Vec<Grain>,
    /// Grains spawned per second.
    grain_rate: f32,
    /// Grain duration in milliseconds.
    grain_duration_ms: f32,
    /// Pitch shift via playback rate.
    pitch_shift: f32,
    /// Current read position in the source (fractional sample).
    time_position: f32,
    /// Time jitter amount (in source samples).
    spray: f32,
    /// Window function for new grains.
    window: GrainWindow,
    /// Sample rate in Hz.
    sample_rate: f32,
    /// Accumulator for grain spawning.
    spawn_accumulator: f32,
    /// Simple PRNG state for spray jitter.
    rng_state: u32,
}

impl GranularEngine {
    /// Create a new granular engine with no source loaded.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            source: Vec::new(),
            grains: vec![Grain::default(); MAX_GRAINS],
            grain_rate: 20.0,
            grain_duration_ms: 50.0,
            pitch_shift: 1.0,
            time_position: 0.0,
            spray: 0.0,
            window: GrainWindow::Hann,
            sample_rate,
            spawn_accumulator: 0.0,
            rng_state: 42,
        }
    }

    /// Load a source audio buffer.
    pub fn set_source(&mut self, buffer: Vec<f32>) {
        self.source = buffer;
        self.time_position = 0.0;
    }

    /// Set the grain spawn rate (grains per second).
    pub fn set_grain_rate(&mut self, rate: f32) {
        self.grain_rate = rate.max(0.1);
    }

    /// Set the grain duration in milliseconds.
    pub fn set_grain_duration(&mut self, ms: f32) {
        self.grain_duration_ms = ms.max(1.0);
    }

    /// Set the playback pitch shift (1.0 = normal pitch).
    pub fn set_pitch_shift(&mut self, shift: f32) {
        self.pitch_shift = shift.max(0.01);
    }

    /// Set the read position in the source (0.0 to 1.0, normalised).
    pub fn set_position(&mut self, pos: f32) {
        if !self.source.is_empty() {
            self.time_position = pos.clamp(0.0, 1.0) * (self.source.len() - 1) as f32;
        }
    }

    /// Set the time jitter (spray) amount in milliseconds.
    pub fn set_spray(&mut self, ms: f32) {
        self.spray = (ms / 1000.0) * self.sample_rate;
    }

    /// Set the window function for new grains.
    pub fn set_window(&mut self, window: GrainWindow) {
        self.window = window;
    }

    /// Generate the next output sample.
    #[inline]
    #[must_use]
    pub fn next_sample(&mut self) -> f32 {
        if self.source.is_empty() {
            return 0.0;
        }

        // Spawn new grains at the configured rate.
        self.spawn_accumulator += self.grain_rate / self.sample_rate;
        while self.spawn_accumulator >= 1.0 {
            self.spawn_accumulator -= 1.0;
            self.spawn_grain();
        }

        // Sum active grains.
        let mut sum = 0.0f32;
        let src_len = self.source.len();

        for grain in &mut self.grains {
            if !grain.active {
                continue;
            }

            // Window position (0..1).
            let t = if grain.duration_samples > 0 {
                grain.current_sample as f32 / grain.duration_samples as f32
            } else {
                0.0
            };
            let window_val = grain.window.value(t);

            // Read from source with linear interpolation.
            let read_pos =
                grain.source_position + grain.current_sample as f32 * grain.playback_rate;
            let read_pos = read_pos.rem_euclid(src_len as f32);
            let idx0 = read_pos.floor() as usize % src_len;
            let idx1 = (idx0 + 1) % src_len;
            let frac = read_pos - read_pos.floor();
            let sample = self.source[idx0] * (1.0 - frac) + self.source[idx1] * frac;

            sum += sample * window_val * grain.amplitude;

            grain.current_sample += 1;
            if grain.current_sample >= grain.duration_samples {
                grain.active = false;
            }
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

    /// Spawn a new grain in the next available slot.
    fn spawn_grain(&mut self) {
        let duration_samples = ((self.grain_duration_ms / 1000.0) * self.sample_rate) as u32;

        // Find an inactive slot index.
        let slot_idx = self.grains.iter().position(|g| !g.active);
        let slot_idx = match slot_idx {
            Some(i) => i,
            None => return, // All slots full — drop this grain.
        };

        let jitter = if self.spray > 0.0 {
            let r = self.next_rng_f32();
            (r - 0.5) * 2.0 * self.spray
        } else {
            0.0
        };

        let src_len = self.source.len() as f32;
        let pos = (self.time_position + jitter).rem_euclid(src_len);

        self.grains[slot_idx] = Grain {
            source_position: pos,
            playback_rate: self.pitch_shift,
            window: self.window,
            duration_samples,
            current_sample: 0,
            amplitude: 1.0,
            active: true,
        };
    }

    /// Simple xorshift PRNG returning f32 in [0, 1).
    #[inline]
    fn next_rng_f32(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = if x == 0 { 1 } else { x };
        (x as f32) / (u32::MAX as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_source_produces_silence() {
        let mut engine = GranularEngine::new(44100.0);
        let mut buf = [0.0f32; 256];
        engine.fill_buffer(&mut buf);
        assert!(
            buf.iter().all(|&s| s == 0.0),
            "empty source should produce silence"
        );
    }

    #[test]
    fn test_loaded_source_produces_output() {
        let mut engine = GranularEngine::new(44100.0);
        // Create a simple sine source.
        let source: Vec<f32> = (0..44100)
            .map(|i| (i as f32 / 44100.0 * 440.0 * std::f32::consts::TAU).sin())
            .collect();
        engine.set_source(source);
        engine.set_grain_rate(50.0);
        engine.set_grain_duration(30.0);

        let mut buf = [0.0f32; 4096];
        engine.fill_buffer(&mut buf);
        assert!(
            buf.iter().any(|&s| s.abs() > 0.01),
            "loaded source should produce output"
        );
        assert!(buf.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_window_functions() {
        // Hann: 0 at endpoints, 1 at centre.
        assert!((GrainWindow::Hann.value(0.0)).abs() < 0.01);
        assert!((GrainWindow::Hann.value(0.5) - 1.0).abs() < 0.01);
        assert!((GrainWindow::Hann.value(1.0)).abs() < 0.01);

        // Gaussian: peak at 0.5.
        assert!(GrainWindow::Gaussian.value(0.5) > GrainWindow::Gaussian.value(0.0));

        // Rectangular: always 1.
        assert!((GrainWindow::Rectangular.value(0.3) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut engine = GranularEngine::new(44100.0);
        engine.set_grain_rate(30.0);
        engine.set_grain_duration(40.0);

        let json = serde_json::to_string(&engine).unwrap();
        let back: GranularEngine = serde_json::from_str(&json).unwrap();
        assert!((engine.grain_rate - back.grain_rate).abs() < f32::EPSILON);
        assert!((engine.grain_duration_ms - back.grain_duration_ms).abs() < f32::EPSILON);
        // Source is skipped in serde, so it should be empty after roundtrip.
        assert!(back.source.is_empty());
    }

    #[test]
    fn test_spray_jitter() {
        let mut engine = GranularEngine::new(44100.0);
        let source: Vec<f32> = (0..44100).map(|i| (i as f32) / 44100.0).collect();
        engine.set_source(source);
        engine.set_spray(10.0); // 10ms jitter
        engine.set_grain_rate(100.0);

        let mut buf = [0.0f32; 1024];
        engine.fill_buffer(&mut buf);
        assert!(buf.iter().all(|s| s.is_finite()));
    }
}
