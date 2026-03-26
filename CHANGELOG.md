# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-26

### Added

- Initial scaffold of naad audio synthesis crate
- `oscillator` module: Waveform enum (Sine, Saw, Square, Triangle, Pulse, WhiteNoise, PinkNoise, BrownNoise), Oscillator struct with PolyBLEP anti-aliasing
- `wavetable` module: Wavetable from raw samples or additive harmonics, WavetableOscillator with linear interpolation, MorphWavetable for crossfading between tables
- `envelope` module: ADSR envelope with linear segments, MultiStageEnvelope with arbitrary segments
- `filter` module: BiquadFilter with Audio EQ Cookbook coefficients (LP, HP, BP, Notch, AllPass, LowShelf, HighShelf, Peak), StateVariableFilter with simultaneous outputs
- `modulation` module: LFO, FmSynth (FM synthesis), RingModulator, ModulationSource trait
- `delay` module: DelayLine with fractional delay, CombFilter, AllpassDelay
- `effects` module: Chorus (multi-tap modulated delay), Flanger (short feedback delay with LFO), Phaser (allpass cascade), Distortion (SoftClip/HardClip/WaveFold)
- `noise` module: NoiseGenerator with White (xorshift32), Pink (Voss-McCartney), Brown (integrated white) noise types
- `tuning` module: equal_temperament_freq, midi_to_freq, freq_to_midi, cents, TuningTable with predefined systems (Equal Temperament, Just Intonation, Pythagorean)
- `error` module: NaadError enum with thiserror derive
- Integration tests: sine period verification, PolyBLEP anti-aliasing check, ADSR sustain hold, biquad -3dB at cutoff, equal temperament A4/C4, FM sideband production, serde roundtrips, pink noise spectral slope
- Criterion benchmarks: oscillator_sine_1024, oscillator_saw_polyblep_1024, biquad_filter_1024, adsr_envelope_1024, fm_synthesis_1024, wavetable_1024
- CI/CD: GitHub Actions for check, security, deny, test, MSRV, coverage, doc, release
- Project documentation: README, CLAUDE.md, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, architecture overview, roadmap
