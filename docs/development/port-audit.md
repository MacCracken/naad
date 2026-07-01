# naad — Rust → Cyrius Port Audit

Per-module parity ledger for the 2.0.0 port. The Rust oracle is frozen at
`rust-old/`; every Cyrius module must match it function-for-function. Update
the relevant row whenever a module's status changes.

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
- **`enum` errors → integer codes**; validators return `ERR_NONE` (0) or a
  negative `ERR_*` from `src/error.cyr`. `Option<Error>` → the code directly.
- **`Result<T>` / `Option<T>`** → sentinel returns (error code, or a NaN/None
  sentinel) unless a real payload is needed (then `lib/tagged.cyr`).
- **`Vec<T>` → stdlib `vec`** (`vec_new`/`vec_push`/`vec_len`/`vec_get`/`vec_set`);
  f64 elements store directly in the 8-byte slots. `SmallVec` → `vec` too.
- **`&mut [f32]` / `&[f32]`** buffers → a `vec` handle (mutated in place / read).
- **structs** via `#derive(accessors)` + `alloc(sizeof(T))`; methods become free
  functions `TypeName_verb(self, …)`. Fixed `[f32; N]` inline arrays → a `vec`
  or a manual byte-offset layout as the Rust shape dictates.
- **Free functions get a `<module>_` prefix** (`filter_process_sample`,
  `delay_read`) — the bundle is one flat namespace; front-load collision
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

- cycc pin: **6.3.18** (`cyrius.cyml [package].cyrius`).
- Deps: hisab (path `../hisab`, `dist/hisab.cyr`) for HVec3/HComplex/fft;
  goonj wired in at the acoustics layer.
- Build: `cyrius build src/main.cyr build/naad`
- Test ONE suite: `cyrius test tests/<mod>.tcyr` (explicit path — no discovery).
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
| additive (synth)|  411 | ✅ 21 | (error), hisab | Additive/DCT via hisab `num_dct`/`num_idct`; ERR_COMPUTATION on failure. |
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

### L-acoustics — wrappers over goonj (wire `[deps.goonj]` first)

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
zero deferrals.** **463 parity assertions green** across 36 suites (all Rust
`#[test]` blocks ported one-for-one minus serde/Display). The 3 `mod.rs` files
carry no logic. Remaining before the 2.0.0 tag: `dist/naad.cyr` bundle +
collision audit, benchmarks, CHANGELOG, VERSION bump. `lib.rs` (module organization +
`flush_denormal`, folded into `error.cyr`) and the per-dir `mod.rs` files
(feature-gated `pub mod` only) carry no independent logic to port.

## Deferred / follow-up work

- **dsp_util hisab-interop block** — the 8 spectral/spline functions above.
- **`[deps.goonj]`** — wire before the acoustics layer (pulls sakshi transitively
  for goonj's `logging`).
- **`[lib]` distlib bundle** (`dist/naad.cyr`) — assemble in dependency order
  once modules land; cross-module symbol-collision audit at close-out.
- **Version 2.0.0** — bump `VERSION` at port completion (per user directive:
  "version project 2.0.0 after port process"). Held at 1.2.5 during the port.
