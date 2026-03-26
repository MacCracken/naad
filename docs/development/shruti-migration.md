# Shruti → naad Migration Guide

## Overview

This document maps shruti-instruments inline DSP code to naad types. The migration replaces ~8,600 lines of synthesis code in shruti-instruments with naad dependency calls.

## Type Mapping

| shruti-instruments | naad | Notes |
|---|---|---|
| `Oscillator` (522 LOC) | `naad::oscillator::Oscillator` | PolyBLEP, waveform enum match. naad adds 4-point PolyBLEP. |
| `Oscillator` + unison loop | `naad::oscillator::UnisonOscillator` | naad has built-in unison with stereo spread. Replaces per-voice detune loop in synth.rs. |
| `Filter` (492 LOC) | `naad::filter::StateVariableFilter` | Both use Cytomic topology. naad caches coefficients. |
| `Envelope` (413 LOC) | `naad::envelope::Adsr` | naad stores sample_rate internally. |
| `Lfo` (208 LOC) | `naad::modulation::Lfo` | naad has 6 shapes (vs shruti's 6). `LfoShape` enum maps directly. |
| `ModMatrix` (482 LOC) | `naad::mod_matrix::ModMatrix` | 16 routing slots. Source/destination enums match shruti's. |
| `VoiceManager` (240 LOC) | `naad::voice::VoiceManager` | Poly/mono/legato, steal modes match. |
| `EffectChain` (761 LOC) | `naad::effects::*` + `naad::reverb::Reverb` | Individual effects compose. |
| `SubtractiveSynth` render loop | `naad::synth::subtractive::SubtractiveSynth` | Single-voice chain. dhvani handles polyphony. |

## What Stays in shruti-instruments

- `InstrumentNode` trait — DAW-specific interface
- `InstrumentPreset` — JSON preset serialization
- `SynthParam` enum — DAW parameter indexing (maps to naad setter calls)
- Step sequencer UI logic
- Plugin hosting (VST3/CLAP)
- Drum machine pattern management (uses `naad::synth::drum` for sound)
- Sampler zone mapping (sample playback stays in dhvani)

## Migration Steps

1. Add `naad = "0.5"` to shruti-instruments Cargo.toml
2. Replace `crate::oscillator::Oscillator` with `naad::oscillator::Oscillator`
3. Replace `crate::filter::Filter` with `naad::filter::StateVariableFilter`
4. Replace `crate::envelope::Envelope` with `naad::envelope::Adsr`
5. Replace `crate::lfo::Lfo` with `naad::modulation::Lfo`
6. Replace inline unison loop with `naad::oscillator::UnisonOscillator`
7. Replace `crate::mod_matrix` with `naad::mod_matrix::ModMatrix`
8. Replace `crate::voice::VoiceManager` with `naad::voice::VoiceManager`
9. Map `SynthParam` setters to naad accessor methods
10. Replace `render_voices` with per-voice `SubtractiveSynth::next_sample()` calls

## Parameter Mapping Example

```rust
// Before (shruti):
match param {
    SynthParam::FilterCutoff => self.filters[voice].cutoff = value,
    SynthParam::FilterResonance => self.filters[voice].resonance = value,
    // ...
}

// After (naad):
match param {
    SynthParam::FilterCutoff => {
        let _ = synth_voices[voice].set_cutoff(value);
    }
    SynthParam::FilterResonance => {
        // resonance mapped via filter.set_params()
    }
    // ...
}
```

## Expected Results

- shruti-instruments shrinks from ~8,600 LOC to ~2,000 LOC (presets, UI mapping, DAW glue)
- All DSP bugs fixed once in naad, shared across consumers
- New synthesis engines (FM, granular, physical, etc.) available to shruti for free
