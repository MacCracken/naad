# naad Roadmap — Pure Audio Synthesis Primitives

> **Version**: 0.1.0 | **Last Updated**: 2026-03-26
> **Status**: Phase 0 complete (initial scaffold) — hardening + expansion ahead

## Vision

naad provides every low-level synthesis building block the AGNOS audio stack needs. **dhvani** composes these into a sound engine (graph, transport, metering, I/O). **svara** handles voice and format concerns. **shruti** is the DAW. naad owns the math that makes sound — nothing more.

### Boundary Rules

| Belongs in naad | Does NOT belong in naad |
|----------------|------------------------|
| Oscillators, filters, envelopes, LFOs | Audio graph, RT-safe node scheduling |
| Wavetables, noise generators, tuning | Transport, clock, metering |
| Modulation routing (mod matrix) | MIDI routing, CC mapping |
| Voice management (polyphony, stealing) | Buffer I/O, resampling, format conversion |
| Dynamics (compressor, limiter, gate) | Preset management, UI parameter mapping |
| EQ, reverb, delay, chorus, panning | Step sequencer, drum patterns |
| Synthesis algorithms (granular, physical modeling, additive, vocoder, formant) | Instrument traits, plugin hosting |
| Acoustics (convolution reverb, room sim, binaural, ambisonics — via goonj) | File I/O (WAV, FLAC, SFZ, SF2) |
| dB/amplitude math, interpolation, denormal flush | |

### Dependency Policy

- `serde` + `thiserror` + `tracing` (already present)
- `hisab` for linear algebra / numerical methods (when needed for FFT, matrix ops, interpolation)
- `goonj` for acoustics (feature-gated: `acoustics`) — room simulation, impulse responses, binaural, ambisonics
- No audio I/O crates. No async. No allocator tricks. Pure computation.

---

## Completed Phases

| Phase | Goal | Key Deliverables |
|-------|------|-----------------|
| 0 — Scaffold | Initial synthesis primitives | Oscillator (PolyBLEP), Wavetable + MorphWavetable, ADSR + MultiStageEnvelope, BiquadFilter + SVF, LFO + FM + RingMod, DelayLine + Comb + Allpass, Chorus + Flanger + Phaser + Distortion, Noise (White/Pink/Brown), Tuning (ET/JI/Pythagorean), Error types, benchmarks, integration tests |
| 1 — Hardening | Audit + fix scaffold | WaveFold fix, Phaser validation, SVF coefficient caching, AllpassDelay single-buffer, denormal protection, Oscillator encapsulation, ADSR stored sample_rate, 14 new serde tests, 8 new benchmarks, gain_db constructor |
| 2 — Primitives | Enhanced oscillator + LFO | 4-point PolyBLEP, HardSync, UnisonOscillator (1-8 voices, stereo), SubOscillator (-1/-2 oct), LFO 6 shapes + bipolar/unipolar modes, SVF buffer methods |
| 3 — New Modules | Dynamics, EQ, reverb, voice, mod matrix, acoustics | dsp_util, Compressor/Limiter/NoiseGate, ParametricEq/GraphicEq/DeEsser, Schroeder Reverb, Panning, ParamSmoother, VoiceManager, ModMatrix + goonj acoustics (room/convolution/binaural/FDN/analysis/ambisonics) |
| 4 — Synthesis | 8 synthesis algorithms | Subtractive, FM (multi-op), Drum (kick/snare/hihat), Formant (vowels), Additive (64 partials), Vocoder, Granular (64 grains), Physical (Karplus-Strong + Waveguide) |
| 5 — Polish | Performance + features + docs | Feature gates (synthesis/acoustics/logging/full), deferred audit fixes (Nyquist, is_active, hermite, Q scaling), 20 benchmarks, allocation audit (clean), SIMD-ready docs, VERSION 0.5.0 |

---

## ~~Phase 1 — Scaffold Hardening~~ COMPLETE

> **Effort**: Medium | **Prerequisite**: None
> **Goal**: Audit and harden the 0.1.0 scaffold before building on it.

Per CLAUDE.md P(-1) process:

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Full test + benchmark sweep | Small | Run all tests, verify all benchmarks produce sane numbers |
| 2 | Cleanliness check | Small | `fmt --check`, `clippy -D warnings`, `cargo audit`, `cargo deny check`, `rustdoc -D warnings` |
| 3 | Baseline benchmarks | Small | `./scripts/bench-history.sh` — establish reference numbers |
| 4 | Internal deep review | Medium | Gaps, correctness, numerical stability, error handling, serde coverage |
| 5 | External research | Medium | Domain best practices — compare against fundamentals, dasp, Faust, SuperCollider |
| 6 | Findings implementation | Medium | Fix issues, add missing tests/benchmarks from review |
| 7 | Post-review benchmarks | Small | Prove no regressions |

---

## ~~Phase 2 — Primitive Enhancements~~ COMPLETE

> **Effort**: Medium | **Prerequisite**: Phase 1
> **Goal**: Bring existing modules up to production quality by incorporating proven patterns from shruti-instruments.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 2A | **Oscillator: 4-point PolyBLEP** | Small | Upgrade from 2-point to 4-point polynomial for steeper rolloff. Shruti already validated this. |
| 2B | **Oscillator: hard sync** | Small | Reset slave phase on master zero-crossing. Pure waveform primitive. |
| 2C | **Oscillator: unison engine** | Medium | N-voice (1-8) with detune spread, stereo width, phase randomization. Precompute detune ratios per buffer (shruti P4 optimization). |
| 2D | **Oscillator: sub-oscillator** | Small | Octave-divided oscillator (-1/-2 oct) with independent waveform. |
| 2E | **LFO: additional shapes** | Small | Add SawUp, SawDown, SampleAndHold to existing Sine/Saw/Square/Triangle. Bipolar (-1..+1) and unipolar (0..+1) output modes. |
| 2F | **SVF: Cytomic/Simper topology** | Medium | Replace Chamberlin SVF with Cytomic topology — better numerical stability at high resonance and high frequencies. Shruti validated this. |
| 2G | **Envelope: coefficient caching** | Small | Cache stage durations at trigger time, not per-tick (shruti P5). |
| 2H | **Denormal flushing utility** | Small | `flush_denormals(f32) -> f32` and buffer variant. Critical for filter/reverb feedback loops (shruti P9). |

---

## ~~Phase 3 — New Primitive Modules~~ COMPLETE

> **Effort**: Large | **Prerequisite**: Phase 2
> **Goal**: Add the missing synthesis primitives that shruti-instruments and shruti-dsp currently provide inline.

### Traditional Primitives (no external deps)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 3A | **Voice manager** | Medium | Polyphony modes (poly/mono/legato), voice stealing (oldest/quietest/lowest), per-voice state. MIDI 2.0 per-note expression fields (pitch bend, pressure, brightness). |
| 3B | **Modulation matrix** | Medium | General-purpose N-slot routing: source enum, destination enum, depth. Sources: LFO, envelope, velocity, mod wheel, aftertouch, pitch bend. Destinations: pitch, filter cutoff, amplitude, pan, etc. |
| 3C | **Compressor** | Medium | RMS envelope detector, gain computer with soft knee, attack/release, makeup gain. LUT-based dB conversion option (shruti P2). |
| 3D | **Limiter** | Small | Brick-wall limiter with lookahead. Builds on compressor internals. |
| 3E | **Noise gate** | Small | Threshold-based gate with attack/hold/release. |
| 3F | **Parametric EQ** | Medium | N-band parametric EQ — each band is a BiquadFilter with configurable type/freq/gain/Q. |
| 3G | **Graphic EQ** | Small | 10-band ISO center frequencies. Wraps parametric EQ. Preset curves (rock, pop, jazz, etc.). |
| 3H | **Algorithmic reverb** | Large | Schroeder topology (4 comb + 2 allpass) and/or FDN. Pre-delay, decay, damping, stereo width. Uses existing DelayLine + AllpassDelay. |
| 3I | **Stereo panning** | Small | Equal-power panning law (sin/cos). Stereo balance. Vector panning. |
| 3J | **Gain smoothing** | Small | EMA-based parameter smoother. Configurable smoothing time. For click-free parameter changes. |
| 3K | **De-esser** | Small | Bandpass sidechain detection + compression in sibilance range (~4-8 kHz). |
| 3L | **dsp_util module** | Small | `amplitude_to_db`, `db_to_amplitude`, `normalize`, `hard_limit`, `soft_clip_tanh`, interpolation helpers (linear, cubic, hermite). Shared free functions. |

### Advanced Acoustics (feature-gated, `goonj` dependency)

> `goonj` (1.0.0) — acoustics engine for sound propagation, room simulation, and impulse response generation.
> These items are behind the `acoustics` feature flag: `naad = { features = ["acoustics"] }`.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 3M | **Convolution reverb** | Medium | IR-based reverb via `goonj::impulse::generate_ir()`. Room dimensions + materials as parameters. Partitioned convolution for real-time use. |
| 3N | **Room simulation reverb** | Medium | Virtual room acoustics using `goonj` ray tracing. Expose room geometry, surface materials, source/listener position. |
| 3O | **Binaural processing** | Medium | HRTF-based spatialization via `goonj::binaural::generate_binaural_ir()`. Headphone-optimized 3D positioning. |
| 3P | **Acoustic analysis utilities** | Small | Expose `goonj::analysis` metrics (C50, C80, D50, STI, RT60) as measurement tools for reverb quality assessment. |
| 3Q | **FDN reverb (goonj-backed)** | Medium | Use `goonj::fdn::Fdn` as an alternative to the traditional Schroeder reverb (3H). Room-derived FDN parameters. |
| 3R | **Ambisonics encoding** | Medium | B-format encoding/decoding via `goonj::ambisonics::BFormatIr`. First-order ambisonics for spatial reverb sends. |

---

## ~~Phase 4 — Synthesis Algorithms~~ COMPLETE

> **Effort**: Large | **Prerequisite**: Phase 3
> **Goal**: Implement the synthesis algorithm modules from shruti's post-MVP roadmap. These are the engines that dhvani will compose into playable instruments.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 4A | **Subtractive synthesis** | Medium | Multi-oscillator + filter + envelope signal chain as a composable struct. Uses existing oscillator, SVF, ADSR, LFO, mod matrix. Not an "instrument" — a synthesis algorithm. |
| 4B | **FM synthesis** | Medium | Arbitrary operator topology (algorithms). Operator = oscillator + envelope + feedback. 2-op, 4-op, 6-op configurations. Extends existing FmSynth. |
| 4C | **Additive synthesis** | Medium | Partial bank with per-partial frequency, amplitude, phase envelopes. Resynthesis from spectral data. Goes beyond current wavetable harmonics. Candidate for `hisab` FFT. |
| 4D | **Wavetable synthesis (advanced)** | Medium | Multi-frame wavetable scanning, spectral morphing, band-limited table generation (mip-mapped). Extends existing Wavetable + MorphWavetable. |
| 4E | **Granular synthesis** | Large | Grain engine: grain pool, windowing (Hann, Gaussian, Tukey), async/sync modes, spray (time jitter), pitch-shift, time-stretch. GrainCloud struct. |
| 4F | **Physical modeling** | Large | Karplus-Strong (plucked string), digital waveguide (string, tube), bowed string exciter, resonator bank. Uses existing DelayLine + filter. |
| 4G | **Vocoder** | Medium | Analysis filter bank (N-band FFT or bandpass) + synthesis filter bank + envelope followers. Carrier × modulator architecture. |
| 4H | **Formant synthesis** | Medium | Formant filter (parallel bandpass/resonator bank), vowel targets (IPA table), interpolation between vowel shapes. |
| 4I | **Drum synthesis** | Medium | Analog drum models: kick (pitch sweep + body resonance), snare (tone + noise burst), hi-hat (metallic noise + BP filter), clap (noise burst cascade). Not a drum *machine* — synthesis primitives for percussive sounds. |

---

## ~~Phase 5 — Performance + Polish~~ COMPLETE

> **Effort**: Medium | **Prerequisite**: Phase 4
> **Goal**: Optimize hot paths, SIMD-ready patterns, comprehensive benchmarks, documentation.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 5A | **SIMD-ready buffer processing** | Medium | Ensure all `process_buffer` / `fill_buffer` methods work on aligned slices. Document SIMD extension points for dhvani. |
| 5B | **Per-buffer coefficient caching** | Small | All filters, oscillators, envelopes: recompute coefficients only on parameter change, not per-sample. Systematic audit. Includes `FormantSynth::morph` per-sample biquad recompute. |
| 5C | **Benchmark suite expansion** | Medium | Every new module gets criterion benchmarks. Target: <1us per 1024-sample buffer for core primitives. |
| 5D | **Allocation audit** | Small | Zero allocations in all `process_sample` / `next_sample` hot paths. Verify with `#[global_allocator]` counting allocator in tests. Includes converting `FormantFilter` Vec to fixed array. |
| 5E | **Documentation** | Medium | Module-level docs, algorithm references, usage examples for each synthesis algorithm. |
| 5F | **Feature gates** | Small | Optional modules behind feature flags. Default = core primitives. `synthesis` flag for Phase 4 algorithms. `dynamics` for compressor/limiter/gate. `eq` for EQ modules. |
| 5G | **AdditiveSynth Nyquist re-check** | Small | `set_fundamental` and `set_partial` must re-zero partials that exceed Nyquist. Currently only checked at construction. Can cause aliasing if fundamental is raised. |
| 5H | **Missing `is_active()` on synths** | Small | `AdditiveSynth`, `KarplusStrong`, `Waveguide`, `GranularEngine` lack `is_active()`. Consumers doing voice allocation need idle detection. KS/Waveguide: check output amplitude < threshold. Additive/Granular: check if any grains/partials active. |
| 5I | **Granular hermite interpolation** | Small | `GranularEngine` source reading uses linear interpolation. Upgrade to `dsp_util::hermite_interpolate` for less aliasing on pitch-shifted grains. |
| 5J | **Vocoder band Q scaling** | Small | Fixed Q=4.0 for all bands. Ideally Q should scale with band spacing for consistent bandwidth coverage across the spectrum. |

---

## Phase 6 — Integration Validation

> **Effort**: Medium | **Prerequisite**: Phase 5
> **Goal**: Validate that dhvani can consume naad cleanly. Prove the abstraction boundary works.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 6A | **dhvani integration smoke test** | Medium | Build a minimal dhvani prototype that wires naad primitives into an audio graph. Validate API ergonomics. Use `dsp_util::fft_magnitudes` for spectral metering. |
| 6B | **shruti migration proof** | Medium | Demonstrate that shruti-instruments can replace inline synthesis code with naad types. Document migration path. |
| 6C | **API stability audit** | Small | Review all public types for forward compatibility. Ensure `#[non_exhaustive]` on all enums. Ensure no leaky abstractions. |
| 6D | **Partitioned FFT convolution** | Medium | Replace O(N) direct convolution in `acoustics::convolution` with overlap-save partitioned FFT using `hisab::num::fft`. Makes long IRs real-time viable. |
| 6E | **Version 1.0.0 gate** | Small | All phases complete, all tests pass, all benchmarks baselined, CHANGELOG updated, VERSION bumped. |

---

## Synthesis Engine Mapping (shruti roadmap → naad)

How each synthesis engine from shruti's post-MVP roadmap maps to naad modules:

| Shruti Engine | naad Module(s) | dhvani Role | hisab Opportunity |
|--------------|----------------|-------------|-------------------|
| Subtractive synth | `oscillator` + `filter` + `envelope` + `modulation` + `voice` + `synth::subtractive` | Compose into InstrumentNode | — |
| FM synth | `oscillator` + `envelope` + `synth::fm` | Compose into InstrumentNode | — |
| Additive synth | `synth::additive` + `envelope` | Compose into InstrumentNode | H3: DCT compression |
| Wavetable synth | `wavetable` + `envelope` + `filter` + `modulation` | Compose into InstrumentNode | H2: B-spline interpolation |
| Physical modeling | `synth::physical` + `delay` + `filter` + `noise` | Compose into InstrumentNode | H1: RK4 analog modeling |
| Granular synth | `synth::granular` + `envelope` + `effects` | Compose into InstrumentNode | — |
| Vocoder | `synth::vocoder` + `filter` + `envelope` | Compose into InstrumentNode | H8: FFT-based vocoder |
| Drum synth | `synth::drum` + `oscillator` + `noise` + `filter` + `envelope` | Compose into InstrumentNode | — |
| Sampler engine | (sample playback lives in dhvani — needs I/O) | dhvani owns this | — |
| Voice synth | `synth::formant` + `synth::vocoder` + `filter` | Compose into InstrumentNode | H6: Catmull-Rom envelopes |

---

## Post-1.0 — Deeper hisab Integration

> These items deepen naad's use of hisab beyond the current FFT/Vec3 usage.
> Each is a natural extension of an existing module.

| # | Item | hisab Feature | naad Module | Notes |
|---|------|--------------|-------------|-------|
| H1 | **RK4 analog circuit modeling** | `num::ode::rk4()` | `synth::physical` | Use ODE solver for more accurate analog circuit models (Moog ladder filter, diode clipper). Currently uses direct DSP equations. |
| H2 | **B-spline wavetable interpolation** | `calc::splines::bspline_eval()` | `wavetable` | Higher-quality interpolation than cubic hermite for smooth wavetable scanning. Reduces aliasing in `MorphWavetable`. |
| H3 | **DCT for additive compression** | `num::dct()` / `num::idct()` | `synth::additive` | Spectral compression/decompression for efficient partial storage and resynthesis from spectral data. |
| H4 | **Matrix FDN parameters** | `num::linalg` (eigenvalue, Hadamard) | `acoustics::fdn_reverb` | Compute FDN feedback matrix directly instead of delegating to goonj. Orthogonal matrix design for color-free reverb. |
| H5 | **Polynomial filter approximation** | `num::linalg::least_squares_poly()` | `filter` | Fit polynomial approximations to expensive filter transfer functions for faster runtime evaluation. |
| H6 | **Catmull-Rom envelope curves** | `calc::splines::catmull_rom()` | `envelope` | Smooth non-linear envelope shapes beyond linear segments. Catmull-Rom through user-placed control points. |
| H7 | **Newton-Raphson pitch detection** | `num::roots::newton_raphson()` | `dsp_util` (new) | Accurate pitch detection via autocorrelation peak refinement. Useful for tuners, pitch correction. |
| H8 | **Spectral analysis suite** | `num::fft()` + `num::dct()` | `dsp_util` | STFT spectrogram, chromagram, onset detection — spectral analysis primitives for composition tools (svara). |

### Current hisab usage

| hisab Module | naad Usage | Since |
|---|---|---|
| `Vec3` | Room/source/listener positions | Phase 3 (acoustics) |
| `Complex` + `num::fft()` | `fft_magnitudes()`, `power_spectrum()` | Phase 5 |

---

## Version Plan

| Version | Milestone | Phases |
|---------|-----------|--------|
| 0.1.x | Scaffold hardening | Phase 1 |
| 0.2.0 | Enhanced primitives | Phase 2 |
| 0.3.0 | New primitive modules (dynamics, EQ, reverb, voice, mod matrix) | Phase 3 |
| 0.4.0 | Synthesis algorithms (subtractive, FM, additive, granular, physical, vocoder, formant, drum) | Phase 4 |
| 0.5.0 | Performance + polish | Phase 5 |
| 1.0.0 | Stable API — dhvani integration validated | Phase 6 |

---

*Last Updated: 2026-03-26 — Phases 0-5 complete, Phase 6 + post-1.0 planned*
