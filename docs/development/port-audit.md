# naad — Rust → Cyrius Port Record

**This is a closed record, not a live dashboard.** The port finished at 41/41
modules and the ledger below was frozen at the 2.0.0 tag: every row's LOC,
status and per-module assertion count is a snapshot from when that module
landed, and none of them are refreshed per release. The file used to head
itself with "update the relevant row whenever a module's status changes"; that
directive went unhonoured for four releases, which is exactly how its numbers
drifted out of date. Relabelling it as a record is the honest fix — the port
history is worth keeping, the pretence of currency is not.

For live numbers read [`state.md`](state.md); for what changed in which
release, the [CHANGELOG](../../CHANGELOG.md); for the toolchain pin and the
dependency pins, `cyrius.cyml` and `cyrius.lock`. Open work is tracked in
[`roadmap.md`](roadmap.md).

The Rust oracle is frozen at `rust-old/`, and the correctness bar was — and
still is, for any change to a ported module — "matches what Rust did".

**Status:** ✅ ported & tested · 🟡 partial · ⬜ pending
**LOC** = Rust lines (incl. tests) at `rust-old/src/`.

## Conventions established (apply to every module)

- **f32 → f64** everywhere (hisab's `HVec3`/`HComplex` are f64-only; widening
  is forced and improves precision). Test tolerances loosened vs the f32 oracle
  where bit-exactness isn't meaningful (`NAAD_EPSILON` = f32::EPSILON promoted).
- **Float literals**: integers via `f64_from(n)`; non-integers as named
  module-top `var` constants holding the IEEE-754 hex bit pattern with the
  decimal in a comment. Generate with:
  `python3 -c "import struct;print(hex(struct.unpack('<Q',struct.pack('<d',X))[0]))"`.
- **`enum` → integer `var` constants** (see `src/error.cyr`, `src/dsp_util.cyr`).
- **`enum` errors → integer codes**; validators return `NAAD_ERR_NONE` (0) or a
  negative `NAAD_ERR_*` from `src/error.cyr`. `Option<Error>` → the code
  directly. (These were bare `ERR_*` through 2.1.2; renamed in 2.1.3 to
  de-collide from goonj in the flat namespace.)
- **`Result<T>` / `Option<T>`** → sentinel returns (error code, or a NaN/None
  sentinel) unless a real payload is needed (then `lib/tagged.cyr`).
- **`Vec<T>` → stdlib `vec`** (`vec_new`/`vec_push`/`vec_len`/`vec_get`/`vec_set`);
  f64 elements store directly in the 8-byte slots. `SmallVec` → `vec` too.
- **`&mut [f32]` / `&[f32]`** buffers → a `vec` handle (mutated in place / read).
- **structs** via `#derive(accessors)` + `alloc(sizeof(T))`; methods become free
  functions `TypeName_verb(self, …)`. Fixed `[f32; N]` inline arrays → a `vec`
  or a manual byte-offset layout as the Rust shape dictates.
- **Free functions get a `<module>_` prefix** (`filter_biquad_process_sample`,
  `delay_line_read`) — the bundle is one flat namespace; front-load collision
  avoidance. `#derive(accessors)` auto-prefixes field accessors by struct name.
- **`&mut u32` PRNG state** → a pointer to a 64-bit slot (`load64`/`store64`),
  u32 wrap emulated with `& 0xFFFFFFFF` after each `<<` (see `xorshift32`).
- **Module files do NOT `include` each other** — the build/test entry includes
  them in dependency order (stdlib auto-prepends; hisab via `include "lib/hisab.cyr"`).
- **serde round-trip tests dropped** (no serde). Display-string tests dropped
  (integer codes). All other `#[test]` blocks ported one-for-one.
- **Cross-check every module against `rust-old/`** — the correctness bar is
  "matches what Rust did".

## Toolchain & commands

- **Toolchain pin**: `cyrius.cyml [package].cyrius` is the source of truth. Read
  it there — a version number copied into prose goes stale the next bump.
- **Deps**: hisab (`dist/hisab.cyr` — HVec3/HComplex/fft; the least-squares
  solver `dsp_spectral` calls is `ganita_mat_least_squares`, which comes from
  the stdlib `ganita` module in `[deps].stdlib`, not from hisab) and
  goonj (`dist/goonj.cyr` — the acoustics engine, which pulls sakshi
  transitively) are both **git+tag pinned** in `cyrius.cyml`; the exact resolved
  commits are recorded in `cyrius.lock`.
- Build: `cyrius build src/main.cyr build/naad`
- Test: bare `cyrius test` auto-discovers and runs **every** `tests/**/*.tcyr` —
  that is the single test step CI runs. `cyrius test tests/<mod>.tcyr` runs one
  suite, which is what you want when iterating on a single module. Both forms
  are real; neither is a substitute for the other.
- **Concurrency**: `cyrius test`/`build`/`deps` re-resolve deps and race on
  `cyrius.lock`. Parallel porting agents MUST serialize every `cyrius …` call
  behind a shared file lock: `flock <scratch>/naad-build.lock cyrius test …`.

## Ledger

### L0 — base (no non-error internal deps)

| Module    |  LOC | Status | Tests | Notes |
|-----------|-----:|--------|------:|-------|
| error     |  192 | ✅ | 23 | Integer codes + `flush_denormal`, `naad_is_finite`, `NAAD_EPSILON/POS_INF/NEG_INF/F32_MIN_POS`, `validate_frequency/sample_rate`. The universal base — every entry includes it first. Folds in the `lib.rs` crate-root helpers. |
| dsp_util  | 1218 | ✅ | 36 | Scalar core: dB↔amp, normalize/rms/peak, clip/lerp/hermite/crossfade, hann/blackman windows, `eval_polynomial`, dB LUT, xorshift32 PRNG, enums. (The `synthesis`-gated spectral block split into `dsp_spectral.cyr` — needs hisab, most dsp_util consumers don't.) |
| dsp_spectral | — | ✅ | 40 | The former dsp_util hisab-interop block: `fft_magnitudes`, `power_spectrum`, `stft_magnitudes`, `chromagram`, `detect_onsets`, `detect_pitch_autocorr` (inline Newton), `bspline_eval_1d` (hisab `calc_bspline`), `fit_polynomial` (Vandermonde + `ganita_mat_least_squares`). |
| panning   |  131 | ✅ | 12 | `PanLaw`→consts; tuple returns → `PanPair`/`PanGains` structs. |
| mod_matrix|  312 | ✅ |  5 | `ModSource`/`ModDest`→index consts (value==slot); vec of `ModRouting`. |
| tuning    |  309 | ✅ | 13 | `TuningSystem`→consts, `TuningTable` struct; div_euclid/rem_euclid over 12; cstring note names via fmt. |
| voice     |  310 | ✅ | 10 | `Voice`/`VoiceManager`; steal modes; `VOICE_NONE=-1`/null sentinels; u64 age in i64 slot. |
| smoothing |  146 | ✅ |  4 | ParamSmoother EMA. NOTE: oracle has no linear mode (dsp_util SMOOTHING_* unused). |
| delay     |  217 | ✅ |  5 | delay line / ring buffer; fractional read via dsp_util interp; flush_denormal. |
| filter    |  516 | ✅ | 13 | BiquadFilter + FilterType→consts + RK4 Moog ladder; validate_* from error. |
| noise     |  222 | ✅ |  8 | NoiseGenerator white/pink(Voss-McCartney 16-oct)/brown; xorshift32 pointer-state. |

### L1 — depend on L0 (non-error)

| Module          |  LOC | Status | Deps | Notes |
|-----------------|-----:|--------|------|-------|
| granular (synth)|  508 | ✅ 11 | dsp_util | granular engine; xorshift spray. Ported in Wave 1. |
| reverb          |  303 | ✅  7 | delay | Schroeder: predelay→4 combs→2 allpass→stereo/mix. |
| eq              |  343 | ✅  9 | dsp_util, filter | Parametric/Graphic EQ + DeEsser; SmallVec→vec of biquads. |
| formant (synth) |  336 | ✅  5 | filter | Vowel formant filter banks; tuple→FormantBand struct. |
| vocoder (synth) |  249 | ✅  9 | filter | Channel vocoder; SmallVec→vec bands. |
| drum (synth)    |  501 | ✅ 12 | dsp_util, filter | Kick/snare/hat; xorshift + biquad. |
| envelope        |  688 | ✅ 20 | (error), hisab | ADSR + curve morph via hisab `calc_catmull_rom`. |
| additive (synth)|  411 | ✅ 21 | (error), hisab | Additive/DCT via hisab `num_dct`/`num_idct`; `NAAD_ERR_COMPUTATION` on failure. |
| physical (synth)|  637 | ✅ 15 | delay, dsp_util, hisab | Karplus-Strong/waveguide; hisab `num_rk4` derivative via fn-ptr. |
| wavetable       |  505 | ✅ 11 | dsp_util, dsp_spectral, hisab | Full: core + cubic B-spline morph (`next_sample_smooth` → `bspline_eval_1d`). |
| dynamics        |  562 | ✅ 14 | dsp_util | compressor/limiter/gate/LevelDetector; dB LUT hot path. (envelope was a doc-only false dep.) |
| fm (synth)      |  390 | ✅  9 | envelope | FM operators + DX algorithms; SmallVec→vec. |

### L2 — oscillators & spine

| Module               |  LOC | Status | Deps | Notes |
|----------------------|-----:|--------|------|-------|
| oscillator/core      |  403 | ✅ | noise, unison* | → `src/osc_core.cyr`. Waveform→consts; PolyBLEP; embeds a real `NoiseGenerator` (noise.cyr) so Pink=Voss-McCartney/Brown=integrated — **parity fix applied at integration** (agent had emitted white for all three). |
| oscillator/unison    |  309 | ✅ | core, dsp_util | → `src/osc_unison.cyr`. 8-voice detune + stereo spread. |
| oscillator/sub       |  152 | ✅ | core | → `src/osc_sub.cyr`. Octave-divided layer; SubOctave→consts. |
| oscillator/sync      |  117 | ✅ | core | → `src/osc_sync.cyr`. Master/slave hard sync. |

**All 4 → one `tests/oscillator.tcyr` (27 assertions).** core↔unison mutual dep
handled by the flat-namespace bundle (both in the test unit).

### L3 — routing & composite synths

| Module               |  LOC | Status | Deps | Notes |
|----------------------|-----:|--------|------|-------|
| modulation           |  460 | ✅ 8 | oscillator | Lfo/FmModulator/RingModulator. (`synth` was a doc-only false dep.) |
| effects              |  399 | ✅ 7 | delay, oscillator | Chorus/Flanger/Phaser/Distortion. NOTE: `mod` is a Cyrius reserved word. |
| subtractive (synth)  |  240 | ✅ 5 | envelope, filter, oscillator | Osc→SVF→ADSR voice. |

### L-acoustics — wrappers over goonj (gated on `[deps.goonj]`, wired in M4)

`material_by_name` is a shared helper in `acoustics/mod.rs`; port it to a base
`acoustics.cyr`. All consume goonj's `dist/goonj.cyr` (+ hisab HVec3).

| Module                  |  LOC | Status | Deps (goonj + …) | Notes |
|-------------------------|-----:|--------|------------------|-------|
| acoustics (mod helper)  |   34 | ✅  8 | goonj/material | `naad_acoustics_material_by_name` → `src/acoustics.cyr` (7-name match). |
| acoustics/ambisonics    |  241 | ✅ 14 | goonj/hisab | → `acoustics_ambisonics.cyr`. B-format encode over goonj. |
| acoustics/directivity   |  154 | ✅ 16 | goonj/hisab | → `acoustics_directivity.cyr`. Omni/cardioid/fig-8; polar closed forms. |
| acoustics/room          |  208 | ✅  5 | goonj/hisab, acoustics | → `acoustics_room.cyr`. RoomReverb(Config); goonj `acoustic_room_shoebox`+`generate_ir`. |
| acoustics/binaural      |  260 | ✅  5 | goonj/hisab | → `acoustics_binaural.cyr`. HRTF spatialization; goonj binaural bundle. |
| acoustics/fdn_reverb    |  521 | ✅ 13 | goonj, error | → `acoustics_fdn.cyr`. goonj `fdn_config_for_room`/`fdn_new`/`fdn_process_*`. |
| acoustics/convolution   |  297 | ✅ 17 | goonj/hisab, room | → `acoustics_convolution.cyr`. FFT convolution via hisab `num_fft`/`num_ifft` (interleaved complex). |
| acoustics/analysis      |  273 | ✅ 17 | goonj, room | → `acoustics_analysis.cyr`. C50/C80/D50/STI + absorption advice. |
| acoustics/coupled       |  228 | ✅  9 | goonj, room | → `acoustics_coupled.cyr`. Two-room + portal double-slope decay. |

**Totals:** 41 Rust modules · 13,465 lines. **✅ PORT COMPLETE — 41/41 modules,
zero deferrals.** **463 parity assertions green across 36 suites, as of the
2.0.0 tag** (all Rust `#[test]` blocks ported one-for-one minus serde/Display).
The suite has grown in every release since; `state.md` carries the current
count. Outstanding at the moment this ledger closed — **all four shipped in
2.0.0** — were: `dist/naad.cyr` bundle + collision audit, benchmarks,
CHANGELOG, VERSION bump. `lib.rs` (module organization + `flush_denormal`) was
folded into `error.cyr`. Of the 3 per-dir `mod.rs` files, only
`acoustics/mod.rs` carried logic — `material_by_name`, ported as the base
`acoustics.cyr` row above; `oscillator/mod.rs` and `synth/mod.rs` are
feature-gated `pub mod` + re-exports with nothing independent to port.

## Deferred / follow-up work — all closed in 2.0.0

Kept as a record of what the port deferred and when it came back. Live open
work is in [`roadmap.md`](roadmap.md), not here.

- ✅ **dsp_util hisab-interop block** — landed as `src/dsp_spectral.cyr` (the 8
  spectral/spline functions above).
- ✅ **`[deps.goonj]`** — wired ahead of the acoustics layer; pulls sakshi
  transitively for goonj's `logging`. Since pinned to a released tag alongside
  hisab (see `cyrius.cyml` / `cyrius.lock`).
- ✅ **`[lib]` distlib bundle** (`dist/naad.cyr`) — assembled in dependency
  order, cross-module symbol-collision audit clean at close-out. That audit was
  **fn-scoped** and provably could not see top-level `var` collisions, which is
  how the `ERR_*` shadowing of goonj's codes survived to 2.1.3; it was closed
  there by the `NAAD_ERR_` rename, and the audit now covers `var`s too.
- ✅ **Version 2.0.0** — `VERSION` bumped at port completion (per user
  directive: "version project 2.0.0 after port process"), having been held at
  1.2.5 throughout the port.
