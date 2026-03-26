//! ADSR envelope generator and multi-stage envelopes.
//!
//! Provides standard Attack-Decay-Sustain-Release envelopes with linear
//! segments, plus a flexible multi-stage envelope for arbitrary shapes.

use serde::{Deserialize, Serialize};

use crate::error::{NaadError, Result};

/// Envelope state machine stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EnvelopeState {
    /// Envelope is inactive (output = 0).
    Idle,
    /// Attack phase (rising from 0 to 1).
    Attack,
    /// Decay phase (falling from 1 to sustain level).
    Decay,
    /// Sustain phase (holding at sustain level).
    Sustain,
    /// Release phase (falling from current to 0).
    Release,
}

/// ADSR envelope generator with linear segments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adsr {
    /// Attack time in seconds.
    pub attack_time: f32,
    /// Decay time in seconds.
    pub decay_time: f32,
    /// Sustain level (0.0 to 1.0).
    pub sustain_level: f32,
    /// Release time in seconds.
    pub release_time: f32,
    /// Current envelope state.
    pub state: EnvelopeState,
    /// Current output value.
    current_value: f32,
    /// Value at the start of the release phase.
    release_start_value: f32,
    /// Time spent in the current stage (in samples).
    stage_samples: f32,
}

impl Adsr {
    /// Create a new ADSR envelope.
    ///
    /// All times are in seconds. Sustain level is 0.0 to 1.0.
    ///
    /// # Errors
    ///
    /// Returns error if any time is negative or sustain is out of range.
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Result<Self> {
        if attack < 0.0 {
            return Err(NaadError::InvalidParameter {
                name: "attack".to_string(),
                reason: "must be >= 0".to_string(),
            });
        }
        if decay < 0.0 {
            return Err(NaadError::InvalidParameter {
                name: "decay".to_string(),
                reason: "must be >= 0".to_string(),
            });
        }
        if !(0.0..=1.0).contains(&sustain) {
            return Err(NaadError::InvalidParameter {
                name: "sustain".to_string(),
                reason: "must be between 0.0 and 1.0".to_string(),
            });
        }
        if release < 0.0 {
            return Err(NaadError::InvalidParameter {
                name: "release".to_string(),
                reason: "must be >= 0".to_string(),
            });
        }

        Ok(Self {
            attack_time: attack,
            decay_time: decay,
            sustain_level: sustain,
            release_time: release,
            state: EnvelopeState::Idle,
            current_value: 0.0,
            release_start_value: 0.0,
            stage_samples: 0.0,
        })
    }

    /// Trigger the envelope (note on).
    pub fn gate_on(&mut self) {
        self.state = EnvelopeState::Attack;
        self.stage_samples = 0.0;
    }

    /// Release the envelope (note off).
    pub fn gate_off(&mut self) {
        if self.state != EnvelopeState::Idle {
            self.release_start_value = self.current_value;
            self.state = EnvelopeState::Release;
            self.stage_samples = 0.0;
        }
    }

    /// Generate the next envelope value.
    ///
    /// Returns a value between 0.0 and 1.0.
    #[inline]
    pub fn next_value(&mut self, sample_rate: f32) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.current_value = 0.0;
            }
            EnvelopeState::Attack => {
                let attack_samples = self.attack_time * sample_rate;
                if attack_samples <= 0.0 {
                    self.current_value = 1.0;
                    self.state = EnvelopeState::Decay;
                    self.stage_samples = 0.0;
                } else {
                    self.current_value = self.stage_samples / attack_samples;
                    self.stage_samples += 1.0;
                    if self.current_value >= 1.0 {
                        self.current_value = 1.0;
                        self.state = EnvelopeState::Decay;
                        self.stage_samples = 0.0;
                    }
                }
            }
            EnvelopeState::Decay => {
                let decay_samples = self.decay_time * sample_rate;
                if decay_samples <= 0.0 {
                    self.current_value = self.sustain_level;
                    self.state = EnvelopeState::Sustain;
                } else {
                    let progress = self.stage_samples / decay_samples;
                    self.current_value = 1.0 + (self.sustain_level - 1.0) * progress;
                    self.stage_samples += 1.0;
                    if self.current_value <= self.sustain_level {
                        self.current_value = self.sustain_level;
                        self.state = EnvelopeState::Sustain;
                    }
                }
            }
            EnvelopeState::Sustain => {
                self.current_value = self.sustain_level;
            }
            EnvelopeState::Release => {
                let release_samples = self.release_time * sample_rate;
                if release_samples <= 0.0 {
                    self.current_value = 0.0;
                    self.state = EnvelopeState::Idle;
                } else {
                    let progress = self.stage_samples / release_samples;
                    self.current_value = self.release_start_value * (1.0 - progress);
                    self.stage_samples += 1.0;
                    if self.current_value <= 0.0 {
                        self.current_value = 0.0;
                        self.state = EnvelopeState::Idle;
                    }
                }
            }
        }

        self.current_value
    }

    /// Check if the envelope is active (not idle).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state != EnvelopeState::Idle
    }
}

/// A single segment in a multi-stage envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeSegment {
    /// Target level for this segment (0.0 to 1.0).
    pub target: f32,
    /// Duration in seconds.
    pub duration: f32,
}

/// Multi-stage envelope with arbitrary segments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStageEnvelope {
    /// The segments of the envelope.
    pub segments: Vec<EnvelopeSegment>,
    /// Current segment index.
    current_segment: usize,
    /// Current output value.
    current_value: f32,
    /// Start value of current segment.
    segment_start_value: f32,
    /// Time spent in current segment (in samples).
    stage_samples: f32,
    /// Whether the envelope is active.
    active: bool,
}

impl MultiStageEnvelope {
    /// Create a new multi-stage envelope.
    ///
    /// # Errors
    ///
    /// Returns error if segments is empty.
    pub fn new(segments: Vec<EnvelopeSegment>) -> Result<Self> {
        if segments.is_empty() {
            return Err(NaadError::InvalidParameter {
                name: "segments".to_string(),
                reason: "must have at least one segment".to_string(),
            });
        }

        Ok(Self {
            segments,
            current_segment: 0,
            current_value: 0.0,
            segment_start_value: 0.0,
            stage_samples: 0.0,
            active: false,
        })
    }

    /// Start the envelope.
    pub fn trigger(&mut self) {
        self.current_segment = 0;
        self.current_value = 0.0;
        self.segment_start_value = 0.0;
        self.stage_samples = 0.0;
        self.active = true;
    }

    /// Generate the next envelope value.
    #[inline]
    pub fn next_value(&mut self, sample_rate: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.current_segment >= self.segments.len() {
            self.active = false;
            return 0.0;
        }

        let seg = &self.segments[self.current_segment];
        let seg_samples = seg.duration * sample_rate;

        if seg_samples <= 0.0 {
            self.current_value = seg.target;
            self.segment_start_value = self.current_value;
            self.current_segment += 1;
            self.stage_samples = 0.0;
        } else {
            let progress = (self.stage_samples / seg_samples).min(1.0);
            self.current_value =
                self.segment_start_value + (seg.target - self.segment_start_value) * progress;
            self.stage_samples += 1.0;

            if self.stage_samples >= seg_samples {
                self.current_value = seg.target;
                self.segment_start_value = self.current_value;
                self.current_segment += 1;
                self.stage_samples = 0.0;
            }
        }

        self.current_value
    }

    /// Check if the envelope is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adsr_basic() {
        let mut env = Adsr::new(0.01, 0.01, 0.5, 0.01).unwrap();
        assert!(!env.is_active());
        env.gate_on();
        assert!(env.is_active());
    }

    #[test]
    fn test_adsr_sustain_holds() {
        let mut env = Adsr::new(0.001, 0.001, 0.7, 0.01).unwrap();
        env.gate_on();
        // Run through attack + decay
        for _ in 0..1000 {
            env.next_value(44100.0);
        }
        // Should be at sustain level
        let val = env.next_value(44100.0);
        assert!(
            (val - 0.7).abs() < 0.01,
            "sustain should hold at 0.7, got {val}"
        );
    }

    #[test]
    fn test_adsr_release_to_zero() {
        let mut env = Adsr::new(0.0, 0.0, 1.0, 0.01).unwrap();
        env.gate_on();
        env.next_value(44100.0);
        env.gate_off();
        for _ in 0..2000 {
            env.next_value(44100.0);
        }
        assert!(!env.is_active());
    }

    #[test]
    fn test_invalid_params() {
        assert!(Adsr::new(-1.0, 0.0, 0.5, 0.0).is_err());
        assert!(Adsr::new(0.0, 0.0, 1.5, 0.0).is_err());
        assert!(Adsr::new(0.0, 0.0, -0.1, 0.0).is_err());
    }

    #[test]
    fn test_multi_stage() {
        let segments = vec![
            EnvelopeSegment {
                target: 1.0,
                duration: 0.01,
            },
            EnvelopeSegment {
                target: 0.5,
                duration: 0.01,
            },
            EnvelopeSegment {
                target: 0.0,
                duration: 0.01,
            },
        ];
        let mut env = MultiStageEnvelope::new(segments).unwrap();
        env.trigger();
        assert!(env.is_active());
        for _ in 0..5000 {
            env.next_value(44100.0);
        }
        assert!(!env.is_active());
    }

    #[test]
    fn test_serde_roundtrip() {
        let env = Adsr::new(0.01, 0.1, 0.5, 0.2).unwrap();
        let json = serde_json::to_string(&env).unwrap();
        let back: Adsr = serde_json::from_str(&json).unwrap();
        assert!((env.attack_time - back.attack_time).abs() < f32::EPSILON);
        assert!((env.sustain_level - back.sustain_level).abs() < f32::EPSILON);
    }
}
