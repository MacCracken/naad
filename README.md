# naad

**naad** (Sanskrit: *primordial sound/vibration*) — Audio synthesis primitives for the AGNOS ecosystem.

Provides foundational building blocks for audio synthesis: oscillators with band-limited waveforms (PolyBLEP), biquad and state variable filters, ADSR and multi-stage envelopes, wavetable synthesis with morphing, FM/ring modulation, delay-based effects (chorus, flanger, phaser, distortion), noise generators (white, pink, brown), and tuning utilities.

## Features

- **Oscillators** — Sine, saw, square, triangle, pulse with PolyBLEP anti-aliasing
- **Filters** — Biquad (Audio EQ Cookbook: LP, HP, BP, notch, allpass, shelves, peak) + state variable filter
- **Envelopes** — ADSR with linear segments + arbitrary multi-stage envelopes
- **Wavetables** — Additive synthesis, linear interpolation, morphing between tables
- **Modulation** — LFO, FM synthesis, ring modulation
- **Delay** — Fractional delay lines, feedback comb filters, allpass delays
- **Effects** — Chorus, flanger, phaser, distortion (soft clip, hard clip, wave fold)
- **Noise** — White (xorshift), pink (Voss-McCartney), brown (integrated white)
- **Tuning** — Equal temperament, just intonation, Pythagorean, custom tuning tables, MIDI conversion

## Usage

```rust
use naad::oscillator::{Oscillator, Waveform};
use naad::envelope::Adsr;
use naad::filter::{BiquadFilter, FilterType};

// Create a 440 Hz sine oscillator at 44.1 kHz
let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0)?;

// Create an ADSR envelope
let mut env = Adsr::new(0.01, 0.1, 0.7, 0.3)?;

// Create a low-pass filter at 2 kHz
let mut filter = BiquadFilter::new(FilterType::LowPass, 44100.0, 2000.0, 0.707)?;

// Generate samples
env.gate_on();
let sample = osc.next_sample() * env.next_value(44100.0);
let filtered = filter.process_sample(sample);
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `logging` | Enable `tracing-subscriber` for structured logging output |

## Architecture

```
naad/src/
├── lib.rs          — crate root, re-exports
├── error.rs        — NaadError enum (thiserror)
├── oscillator.rs   — Waveform, Oscillator, PolyBLEP
├── wavetable.rs    — Wavetable, WavetableOscillator, MorphWavetable
├── envelope.rs     — Adsr, MultiStageEnvelope
├── filter.rs       — BiquadFilter, StateVariableFilter
├── modulation.rs   — Lfo, FmSynth, RingModulator
├── delay.rs        — DelayLine, CombFilter, AllpassDelay
├── effects.rs      — Chorus, Flanger, Phaser, Distortion
├── noise.rs        — NoiseGenerator (White, Pink, Brown)
└── tuning.rs       — equal_temperament_freq, midi_to_freq, TuningTable
```

## Consumers

- **dhvani** — AGNOS sound engine (primary consumer)
- **svara** — music composition tools
- **jalwa** — media player (via dhvani)
- **shruti** — digital audio workstation (via dhvani)

## License

GPL-3.0
