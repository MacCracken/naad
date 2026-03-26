# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - Phase 5: Performance + Polish

### Added

- **Feature gates**: `synthesis` (default, Phase 4 algorithms), `acoustics` (goonj), `logging` (tracing-subscriber), `full` (all features). Core primitives always available with `--no-default-features`.
- **`is_active()` on 4 synths**: `AdditiveSynth` (any non-zero partial), `KarplusStrong` (damping state), `Waveguide` (delay line energy), `GranularEngine` (any active grain)
- **AdditiveSynth Nyquist re-check**: `set_fundamental()` and `set_partial()` now zero out partials whose frequency exceeds Nyquist
- **Granular hermite interpolation**: Source reading upgraded from linear to cubic hermite via `dsp_util::hermite_interpolate`
- **Vocoder proportional Q**: Band Q now scales with logarithmic spacing (`1/(exp(step)-1)`) for consistent bandwidth coverage
- **6 new benchmarks**: compressor, reverb, parametric EQ (4-band), subtractive synth, Karplus-Strong (20 total)
- **Architecture docs**: SIMD-readiness documented — all buffer methods work on contiguous `&mut [f32]`, dhvani handles alignment/dispatch

### Changed

- VERSION bumped to 0.5.0 (phases 0-5 complete)
- `synth` module now behind `synthesis` feature flag (default-enabled)
- hisab upgraded from 0.24 to 1.1.0 — now used for FFT, complex numbers, Vec3
- `synthesis` feature now pulls in hisab for FFT/spectral analysis

## [Unreleased] - Phase 3 goonj + Logging

### Fixed (Acoustics Audit)

- **High**: `ConvolutionReverb` non-functional after serde — added `rebuild_from_ir()` and `is_loaded()` methods; documented O(N) performance limitation
- **High**: `BinauralProcessor` non-functional after serde — added `rebuild()` and `is_loaded()` methods for post-deserialization recovery
- **High**: `FdnReverb` had dead `num_delays` parameter with hardcoded room dimensions — replaced with configurable `room_length/width/height` parameters that drive FDN delay topology
- **Medium**: Added `tracing::debug!` instrumentation to all acoustics constructors (room, binaural, FDN) for consistency with core modules

### Added

- **`acoustics` feature flag** — optional goonj-backed advanced acoustics modules:
  - **`acoustics::room`** — `RoomReverb`: shoebox room simulation reverb via goonj ray tracing
  - **`acoustics::convolution`** — `ConvolutionReverb`: IR-based reverb from room simulation or user-provided impulse responses
  - **`acoustics::binaural`** — `BinauralProcessor`: HRTF-based headphone spatialization via goonj binaural
  - **`acoustics::fdn_reverb`** — `FdnReverb`: feedback delay network reverb wrapping goonj FDN with lazy serde reconstruction
  - **`acoustics::analysis`** — `RoomMetrics` (C50, C80, D50, STI, RT60) from goonj analysis functions
  - **`acoustics::ambisonics`** — `AmbisonicsEncoder`, `BFormatSample`: first-order ambisonics encoding (SN3D/ACN)
- **Tracing instrumentation** — `tracing::debug!` events on Oscillator, BiquadFilter, Reverb, Compressor construction; `tracing::warn!` on validation failures (frequency, sample_rate)
- **Error coverage tests** — all 5 `NaadError` variants tested for Display output and serde roundtrip; validation helpers tested for edge cases (0, negative, NaN, Infinity)
- Dependencies: `goonj = "1"` (optional, `acoustics` feature), `hisab = "0.24"` (optional, `acoustics` feature)

## [Unreleased] - Phase 4: Synthesis Algorithms

### Fixed (Phase 4 Audit)

- **High**: `SubtractiveSynth` recomputed SVF filter coefficients every sample — added cutoff delta threshold (>0.5 Hz) to skip redundant `set_params` calls
- **High**: `FmOperator::next_sample` was private — made public so consumers can build custom FM topologies beyond built-in algorithms
- **Medium**: Granular Tukey window trailing taper had discontinuity — corrected formula using `cos(PI * ...)` instead of `cos(TAU * ...)`
- **Medium**: Drum synthesis xorshift PRNG had no zero-state guard — added `x == 0` recovery (xorshift(0) = 0 forever)
- Added `#[must_use]` on all `next_sample` / `process_sample` methods across all 8 synth modules

### Added

- **`synth` module** — 8 synthesis algorithm submodules:
  - **`synth::subtractive`** — `SubtractiveSynth`: single-voice osc(s) → SVF filter → amp/filter ADSR chain, two-oscillator mixing, filter envelope modulation
  - **`synth::fm`** — `FmSynthEngine`: up to 6 operators with `FmAlgorithm` (Serial2, Parallel2, Serial4, Stack4, Custom), operator feedback, per-operator envelopes
  - **`synth::drum`** — `KickDrum` (pitch-swept sine + noise click), `SnareDrum` (sine + bandpass noise), `HiHat` (6 detuned squares through HP+BP)
  - **`synth::formant`** — `FormantSynth` with `Vowel` enum (A/E/I/O/U, IPA formant values), 3-resonator parallel bank, vowel morphing
  - **`synth::additive`** — `AdditiveSynth`: up to 64 partials with per-partial frequency ratio and amplitude, harmonic series default, Nyquist filtering
  - **`synth::vocoder`** — `Vocoder`: N-band channel vocoder with logarithmically-spaced analysis/synthesis bandpass pairs and envelope followers
  - **`synth::granular`** — `GranularEngine`: 64 grain slots, configurable window (Hann/Gaussian/Tukey/Rectangular), spray jitter, pitch shift, source buffer
  - **`synth::physical`** — `KarplusStrong` (plucked string with lowpass damping) and `Waveguide` (bidirectional delay line tube/string model)

## [Unreleased] - Phase 3: New Primitive Modules

### Fixed (Phase 3 Audit)

- **Critical**: `ModMatrix` used `enum as usize` to index fixed arrays — adding a variant to `#[non_exhaustive]` enums would panic. Replaced with explicit `.index()` match methods decoupled from discriminants. Added `NUM_SOURCES`/`NUM_DESTINATIONS` constants.
- **High**: `GraphicEq::set_band_gain` used `GRAPHIC_EQ_FREQUENCIES[index]` directly — wrong when bands are skipped at low sample rates. Now tracks `active_frequencies` vec for correct index mapping.
- **Medium**: `EnvelopeDetector` NaN/Inf input permanently poisoned state — added `is_finite()` guard, non-finite input treated as 0.0
- **Medium**: `ParamSmoother::set_target` NaN poisoned state — non-finite targets now silently ignored
- **Medium**: `NoiseGate` used release coefficient for both opening and closing — split into separate `attack_coeff` (fast, from attack time) and `release_coeff`
- **Medium**: `Limiter` ratio=100 not true brick-wall — changed to `f32::MAX` for effective infinite ratio
- **Medium**: Reverb comb filter lengths (1116, 1188, 1277, 1356) had common factors — replaced with primes (1117, 1187, 1277, 1361) for better diffusion. Allpass lengths similarly upgraded (556→557, 441→443).

### Added

- **`dsp_util` module** — `amplitude_to_db`, `db_to_amplitude`, `normalize`, `hard_limit`, `soft_clip_tanh`, `lerp`, `hermite_interpolate`, `crossfade_equal_power`, `SmoothingMode` enum
- **`dynamics` module** — `EnvelopeDetector` (attack/release), `Compressor` (threshold, ratio, soft knee, makeup gain), `Limiter` (brick-wall with fast attack), `NoiseGate` (threshold, hold, smooth gate)
- **`eq` module** — `ParametricEq` (N-band, wraps BiquadFilter), `GraphicEq` (10-band ISO frequencies), `DeEsser` (bandpass sidechain + compression)
- **`reverb` module** — `Reverb` (Schroeder: 4 damped comb + 2 allpass, pre-delay, stereo width, wet/dry mix, Freeverb-style delay lengths)
- **`panning` module** — `PanLaw` (EqualPower/Linear), `pan_gains()`, `pan_mono()`, `stereo_balance()`
- **`smoothing` module** — `ParamSmoother` (EMA one-pole lowpass, configurable time constant, snap, settled detection)
- **`voice` module** — `VoiceManager` (poly/mono/legato modes), `StealMode` (oldest/quietest/lowest/none), `Voice` (per-note state with MIDI 2.0 fields: pitch_bend, pressure, brightness)
- **`mod_matrix` module** — `ModMatrix` (16-slot routing), `ModSource` (8 sources), `ModDestination` (8 destinations), `ModRouting` (source→dest with depth)
- Roadmap updated: Phase 3 split into traditional primitives (3A-3L) and goonj-backed advanced acoustics (3M-3R, feature-gated)

## [Unreleased] - Phase 2: Primitive Enhancements

### Fixed (Phase 2 Audit)

- **Critical**: `UnisonOscillator` produced 0 Hz after deserialization — `ratios_dirty` now defaults to `true`, `detune_ratios` defaults to `[1.0; 8]` via serde defaults
- **Critical**: `UnisonOscillator` Triangle/Pulse/noise waveforms silently produced saw — replaced match fallback with `stateless_waveform_sample()` helper supporting all waveforms
- **High**: `Lfo` S&H `rng_state` reset to 0 after deserialization (xorshift(0)=0 forever) — serde default now returns 42
- **High**: `Lfo` S&H output 0.0 for entire first cycle — PRNG now initialized at construction, `sh_value` defaults to 0.5 after deser
- **High**: `SubOscillator::set_base_frequency` mutated state before validation — validation now runs first
- **Medium**: `UnisonOscillator` detune spread was 2x the parameter value — removed erroneous `* 2.0` multiplier; `detune_cents=10` now means 10 cents total spread
- **Medium**: `process_sample_lowpass` doc incorrectly claimed efficiency gain — corrected to "convenience method"
- **Low**: `SubOscillator::set_octave` silently discarded errors — now returns `Result<()>`
- Added `#[must_use]` on `Lfo::next_value`, `#[inline]` on `fill_buffer_stereo`
- Added serde roundtrip tests verifying `UnisonOscillator` and `Lfo` S&H work after deserialization

### Added

- **4-point PolyBLEP** — upgraded from 2-point to 4-point polynomial with cubic refinement; better aliasing suppression at high frequencies
- **`HardSync`** struct — master/slave oscillator pair with automatic phase reset on master cycle completion; `next_sample()`, `fill_buffer()`, frequency setters
- **`UnisonOscillator`** — 1-8 voice unison with symmetric cent-based detune spread, precomputed ratios, stereo width via equal-power panning; `next_sample()`, `next_sample_stereo()`, `fill_buffer_stereo()`
- **`SubOscillator`** — octave-divided oscillator (-1 or -2 octaves) with independent waveform and mix level; `SubOctave` enum
- **`LfoShape` enum** — 6 shapes: Sine, Triangle, Square, SawUp, SawDown, SampleAndHold (was 4 via Waveform reuse)
- **`LfoMode` enum** — Bipolar (-1..+1) and Unipolar (0..+1) output modes
- **LFO standalone implementation** — own phase accumulator, shape enum, and S&H PRNG (no longer wraps Oscillator); `from_waveform()` for backward compatibility
- **`StateVariableFilter::process_sample_lowpass()`** — convenience method returning only LP output
- **`StateVariableFilter::process_buffer_lowpass()`** — buffer variant for LP-only processing
- Tests: hard sync, unison (mono, stereo, single voice, serde), sub-oscillator (octave, serde), LFO (all 6 shapes, unipolar mode)

### Changed

- `polyblep()` function now uses 4-point cubic correction (2 samples each side of discontinuity)
- SVF doc comment corrected to reference Cytomic/Simper topology (math was already correct)

## [Unreleased] - P(-1) Scaffold Hardening

### Fixed

- **Critical**: WaveFold distortion infinite loop — replaced iterative fold with analytical triangle-wave formula; safe for NaN/Inf input and O(1) for any drive value
- **High**: Phaser missing frequency validation — `min_freq`/`max_freq` now clamped to 20 Hz..Nyquist; allpass coefficient clamped for numerical stability
- **High**: SVF filter recomputing coefficients per-sample — cached `g`/`k`/`a1`/`a2`/`a3` in struct, recomputed only via `set_params()`
- **High**: AllpassDelay using two delay lines — replaced with single-buffer Schroeder allpass, halving memory usage
- **High**: No denormal protection in filters — added `flush_denormal()` utility, applied to BiquadFilter and SVF state variables and CombFilter feedback path
- Dead allpass cascade code in Phaser `process_sample` (first loop was overwritten)
- Unused `NaadError` import in oscillator module
- Deprecated `GPL-3.0` SPDX identifier → `GPL-3.0-only` in Cargo.toml and deny.toml
- Unescaped `[n]` in doc comments breaking rustdoc link resolution
- Clippy warnings: collapsible if, unnecessary cast, manual range contains

### Changed

- `Oscillator` fields now private — added `waveform()`, `frequency()`, `phase()`, `sample_rate()`, `pulse_width()` accessors and `set_phase()`, `set_pulse_width()`, `reset_phase()` mutators
- `StateVariableFilter` fields now private — added `frequency()`, `q()`, `sample_rate()` accessors and `set_params()` mutator with validation
- `Adsr` now stores `sample_rate` — `next_value()` takes no arguments; `new()` defaults to 44100 Hz, `with_sample_rate()` for explicit rate
- `MultiStageEnvelope` now stores `sample_rate` — same pattern as Adsr
- `Adsr.state` field now private — added `state()` accessor

### Added

- `BiquadFilter::with_gain()` constructor for shelf/peak filters with `gain_db` parameter
- `Oscillator::advance_phase_sine()` for FM synthesis phase control
- `Oscillator::ensure_initialized()` — lazy noise_gen reconstruction after deserialization
- `MorphWavetable::new()` now validates all tables have matching sample counts
- Phaser `min_freq`/`max_freq` clamped to 20 Hz..Nyquist at construction
- `flush_denormal()` public utility function in crate root
- Serde roundtrip tests for: MultiStageEnvelope, StateVariableFilter, Lfo, RingModulator, Chorus, Flanger, Phaser, Distortion, DelayLine, CombFilter, AllpassDelay, WavetableOscillator, MorphWavetable, NoiseGenerator (14 new integration tests)
- Criterion benchmarks for: SVF filter, white/pink noise, comb filter, allpass delay, chorus, phaser, distortion wavefold (8 new benchmarks, 14 total)
- `#[inline]` on all `fill_buffer` / `process_buffer` methods for cross-crate optimization
- `scripts/bench-history.sh` for benchmark tracking
- `docs/development/roadmap.md` — 6-phase plan from scaffold to 1.0.0

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
