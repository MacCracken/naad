//! # naad — Audio Synthesis Primitives
//!
//! **naad** (Sanskrit: primordial sound/vibration) provides foundational audio
//! synthesis building blocks: oscillators, filters, envelopes, wavetables,
//! modulation, delay lines, effects, noise generators, and tuning utilities.
//!
//! Part of the AGNOS ecosystem. Consumed by **dhvani** (sound engine) and
//! **svara** (music composition).
//!
//! ## Quick Start
//!
//! ```rust
//! use naad::oscillator::{Oscillator, Waveform};
//! use naad::envelope::Adsr;
//! use naad::filter::{BiquadFilter, FilterType};
//!
//! // Create a 440 Hz sine oscillator
//! let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0).unwrap();
//!
//! // Create an ADSR envelope
//! let mut env = Adsr::new(0.01, 0.1, 0.7, 0.3).unwrap();
//!
//! // Create a low-pass filter at 2kHz
//! let mut filter = BiquadFilter::new(FilterType::LowPass, 44100.0, 2000.0, 0.707).unwrap();
//!
//! // Generate a sample
//! env.gate_on();
//! let sample = osc.next_sample() * env.next_value();
//! let filtered = filter.process_sample(sample);
//! ```
//!
//! ## Feature Flags
//!
//! - `logging` — Enable tracing-subscriber for structured logging output

pub mod delay;
pub mod effects;
pub mod envelope;
pub mod error;
pub mod filter;
pub mod modulation;
pub mod noise;
pub mod oscillator;
pub mod tuning;
pub mod wavetable;

pub use error::{NaadError, Result};

/// Flush denormal floating-point values to zero.
///
/// Denormal (subnormal) floats cause 10-100x slowdowns on x86 processors.
/// Call this on filter state variables and feedback paths to prevent
/// performance degradation when signals decay to near-zero.
#[inline]
#[must_use]
pub fn flush_denormal(x: f32) -> f32 {
    // A float is denormal if its exponent bits are all zero but mantissa is non-zero.
    // Threshold: f32::MIN_POSITIVE (1.175e-38) is the smallest normal f32.
    if x.abs() < f32::MIN_POSITIVE { 0.0 } else { x }
}
