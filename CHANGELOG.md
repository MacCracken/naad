# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.2] - Toolchain + dependency catch-up

Fifteen months of upstream in one bump — cyrius **6.3.19 → 6.5.35**, hisab
**2.6.7 → 2.11.2**, goonj **2.0.0 → 2.0.4**, sakshi (transitive) **2.4.2 →
2.4.11** — plus the vendored stdlib re-synced and the port's first post-release
audit of what a dependency bump *silently* changed.

**The suite was green before the bump and green after it, at exactly 477
assertions either side. That is not evidence, and this release is mostly about
what it failed to see.** Every defect below was invisible to it. One is a real
regression this bump introduced: `fit_polynomial` gained a null-pointer
dereference — reproduced as a **SIGSEGV (exit 139)** — because ganita's matrix
constructor learned to report failure and naad's only call site never checked.
The suite is now **502 assertions across 38 suites**, and each of the 25 new
ones was verified to *fail* without its fix.

### Changed — toolchain, dependencies, and the vendored stdlib

- **`cyrius.cyml`**: `cyrius = "6.3.19"` → `"6.5.35"`; `[deps.hisab]` tag
  `2.6.7` → `2.11.2`; `[deps.goonj]` tag `2.0.0` → `2.0.4`. goonj 2.0.4 pins
  hisab 2.11.2 and hisab 2.11.2 pins sakshi 2.4.11, so the graph resolves
  consistently; exact commits are in `cyrius.lock`.
- **`lib/` re-vendored from the 6.5.35 snapshot** — all **30** stdlib files
  verified **byte-identical** to `~/.cyrius/versions/6.5.35/lib`, file by file,
  rather than assumed. (`lib/` holds 32 `.cyr`; the other two, `hisab.cyr` and
  `goonj.cyr`, are the git+tag dependency bundles and have no snapshot
  counterpart.) Comparing old-pin against new-pin would have shown a tidy diff
  and could not have detected a half-synced tree; only comparing against the
  pin's own snapshot can.
- A local hand-edit to `lib/syscalls_x86_64_agnos.cyr` (an `agnos` `sys_fstat`
  peer that fails closed) was discarded rather than re-applied: the same
  function has since landed upstream and arrives with the re-vendored file.
  `naadex.cyr`, the agnos ring-3 oscillator proof, still builds.
- **`dist/naad.deps` was empty and is now populated** with all 15 stdlib leaf
  requirements. Consumers of `dist/naad.cyr` were not being told what naad needs
  in scope — a real consumer-visible fix, independent of the version header.
- **CI gains a pre-tag changelog gate.** `release.yml` builds the release body by
  awk-ing the `## [<tag>]` section out of this file and falls back to the literal
  "No changelog entry for <tag>." — but that step runs *after* the tag is pushed,
  so failing there would leave a published tag with no release. `ci.yml` now runs
  the same extractor against `VERSION` on the branch. Verified in both
  directions: it passes here and fails on a VERSION with no matching section.
  (This release would have tripped it — the entry you are reading did not exist
  when the bump was applied.)
- 12 `src/*.cyr` files reformatted for the 6.5.35 formatter (continuation-line
  indentation). The reformat itself is whitespace only — measured before
  applying it: zero line-count change and zero non-whitespace change across all
  12. Three of those files (`acoustics_analysis`, `acoustics_binaural`,
  `dsp_spectral`) do move line counts in this release, from the code fixes
  below, not from the reformat. `cyrius audit`'s fmt gate is clean again.

### Fixed — a null dereference this bump introduced

- **`fit_polynomial` (`src/dsp_spectral.cyr`) SIGSEGV'd on a negative degree.**
  ganita 1.1.4 gave `ganita_mat_new` failure returns (non-positive dimensions,
  designs over `GANITA_MAT_MAX_ELEMS`, allocation failure) where it had
  previously always returned a usable header. naad's only call site never
  checked it, and `ganita_mat_least_squares` dereferences its argument
  immediately. The existing `nx <= degree` guard cannot catch a negative degree
  — `nx >= 0` is never `<= -1` — so `cols <= 0` reached the constructor.
  **Before** the bump that same path returned a valid 16-byte header and no-op'd
  its way to an empty vec; **after**, it faulted.
  ⚠ **Confirmed by execution, not by reading**: removing the two new guards
  makes `tests/dsp_spectral.tcyr` exit **139**. `degree` is `usize` in
  `rust-old/`, so the oracle cannot express this state at all — the port widened
  the domain to signed `i64` and the bump turned the widened region from benign
  into fatal. No test covered it, which is why 477 assertions passed either side.

### Fixed — half-bound `sample_rate` guards in the acoustics wrappers

Five wrappers tested `sample_rate == 0` where the oracle types the parameter
`u32`. Rust cannot represent a negative rate; signed Cyrius can, and the guards
were transliterated verbatim, so a single upper-bound test was only half a bound.
All five now test `<= 0`, matching naad's own idiom elsewhere
(`src/error.cyr`, `src/vocoder.cyr`, `src/additive.cyr`).

- **`naad_fdn_matrix_new` produced a divergent, unbounded reverb.** A negative
  rate flips the damping exponent positive, making every per-delay gain exceed 1
  (measured 1.000143 at `sr = -48000`); under the orthogonal Hadamard mix the
  loop grows roughly 1000× per 48 000 samples — while the constructor reports
  success. This is the one that produces wrong audio.
- **`naad_analysis_analyze_impulse_response` — bump-attributable.** goonj 2.0.2
  widened `_analysis_clarity`'s own `rate == 0` guard to `rate <= 0`, which
  turned what had been a loud `vec: index < 0` process abort into a fully
  populated `RoomMetrics` carrying a **negative RT60** with `naad_is_err == 0`.
  Every downstream `is_err` and finiteness check passes on wrong data.
- `naad_analysis_estimate_rt60` is a separate public entry point and divided by
  the negative rate directly; it needed its own guard.
- `naad_fdn_reverb_new` and `naad_binaural_new` were benign but silent — goonj
  clamped or returned empty IRs, leaving a processor that reported success and
  passed mono through unspatialised forever. Fixed as input-domain hygiene.

Only the analysis entry point changed behaviour *at this bump*; the other four
are pre-existing port defects this audit surfaced.

### Fixed — a test that could not fail

- `tests/acoustics_analysis.tcyr`'s empty-RT60 assertion compared through
  `f64_to`, which truncates toward zero, so it held for **every** RT60 in
  (−1.0, 1.0) — most of the realistic output range. It now compares raw f64
  bits (+0.0 is the bit pattern 0), matching the oracle's exact equality.

### Changed — inherited behaviour: `f64_tanh` no longer returns NaN

`ganita_f64_tanh` gained saturation guards, so it no longer falls into
`inf/inf` when `f64_exp` overflows.

⚠ **The boundary is |x| > ~709.78, not |x| > 20.** Measured against the old body
compiled standalone: for every integer in **[20, 709] the two paths are
bit-identical** (both exactly 1.0); the first NaN is exactly **x = 710**. ±inf
went NaN → ±1.0. Quoting the guard's literal threshold of 20 would have
described a change that does not exist there, and any test written at drive 100
would have proved nothing.

Three naad surfaces sat directly on it, unvalidated, for **every naad release to
date**:

- `soft_clip_tanh` (`src/dsp_util.cyr`) — public, no validation.
- `effects_distortion_process_sample` under `DISTORTION_SOFT_CLIP` — `drive` is
  floored at 0 and deliberately **not** capped (the oracle is `drive.max(0.0)`
  with no upper bound), so the NaN survived the dry/wet blend.
- `physical_moog_process_sample` — the worst case. `physical_moog_deriv` tanh's
  the input *and* re-tanh's the RK4 stage states, so one over-range sample left
  the ladder NaN for **every subsequent sample until reset**. Persistent state
  corruption, not a per-buffer glitch.

Rust's `tanh` saturates for all inputs and never yields NaN at either width —
`f32::tanh` for `soft_clip_tanh` (`rust-old/src/dsp_util.rs:59`) and the
distortion path (`effects.rs:302`), and `f64::tanh` for the Moog ladder, which
casts to `f64` first (`synth/physical.rs:382`). So **this moves naad toward
parity** — it silently repaired a real divergence. `drive` was
deliberately *not* clamped in response: that would create a fresh divergence to
fix a defect that no longer exists. The new assertions pin the contract at drive
1000 with controls at 100 that pass on both sides of the bump, so a silent
upstream revert cannot reintroduce a NaN nobody is watching for.

*Consumers who relied on the NaN as a signal must adapt.* Note also that
`ganita_f64_sinh` / `ganita_f64_cosh` remain unguarded and still overflow to
±inf; naad does not call them.

### Changed — inherited: DCT/IDCT now dispatch by size

hisab 2.8.3 replaced `num_dct` / `num_idct`'s O(n²) kernels with a size-dispatched
FFT/Bluestein reduction, which `src/additive.cyr`'s amplitude compression crosses
at this bump. Dispatch is non-monotonic: FFT for n = 8, 16, 27–32, 40+; the
retained direct kernel for n = 2–7, 9–15, 17–26, 33–39; first Bluestein at n = 27.
Coefficients moved by ~1e-15 relative — **no parity impact**, and `rust-old/`
delegates to the same hisab entry points. Every existing DCT assertion used
n ∈ {8, 16}, both on the FFT branch, so a dispatch retune was invisible; the
suite now also round-trips n = 5 (direct) and n = 28 (Bluestein).

### Changed — benchmarks are on a new instrument and are not comparable

cyrius 6.5.19 taught `lib/bench.cyr` to **calibrate one clock read on the host
and subtract it from every sample**, and taught `bench_run` to size its own
batches instead of wrapping a clock pair around every iteration. Both rewrite the
number without touching the code being measured.

**2.1.2 makes no speedup claim.** Re-measured on this host (hpet clocksource,
this boot), with the floor stated beside them as it must be:

| Benchmark | 2.1.2 (net of floor) |
|---|---|
| `osc_next_sample` (sine) | 58–64 ns |
| `filter_svf` lowpass | 35–38 ns |
| `envelope_adsr` next_value | 22–23 ns |
| `noise` pink (Voss-McCartney) | 102–110 ns |
| *measured timer floor* | *1.320–1.349 µs per clock read* |

Ranges are the observed envelope over eight runs on one host and one boot, not a
confidence interval — every endpoint is a figure somebody actually saw.

The four operations differ by **4.78×**; 2.0.0 reported them as a single
"~1.4 µs/sample" because a common ~1.34 µs floor swamped all four. A real
regression had room to hide inside that. ⚠ The floor is **host- and
boot-dependent** — `lib/bench.cyr` records this same machine producing both
~400 ns and ~1,700 ns across reboots, and a 230× spread across the four hosts the
upstream gate runs on — so no fixed figure belongs in a comment, and every
recorded row must carry the floor it was taken against.

- `scripts/bench-history.sh` **rewritten**. It was a dead pre-port script running
  `cargo bench` into `benches/history/`; neither exists (no `Cargo.toml`, no
  `benches/`), so it was unreachable from both ends and naad has **zero**
  archived bench rows. The replacement drives `cyrius bench` under the project's
  `flock` rule and writes a CSV carrying **derived** `regime` and `floor_ns`
  columns — read from whether the harness printed its own measured floor, so
  they cannot go stale the way a hand-written constant does. A trend filter must
  refuse to compare across regimes: rows either side of this boundary are all
  `stat=avg`, so filtering on the statistic alone would report pure instrument
  artefact as improvement.
- `tests/naad.bcyr` quoted "~240 ns per start/stop pair on x86_64 Linux", copied
  from the old `lib/bench.cyr` header. Upstream **retired** that figure and
  measures 1,346 ns here — 5.6× low — so the constant is gone and the comment
  points at `bench_clock_overhead_ns()` instead. The "batch sub-microsecond ops"
  guidance it sat next to is still correct and stays.

### Documentation

The docs had drifted far enough that several were describing a project that no
longer exists. Corrected against **measured** state, with dated records left
intact and stamped rather than rewritten.

- **`README.md` was still the pre-port Rust crate README** — a `use
  naad::oscillator::…` usage block, a Feature Flags table whose only row was
  `tracing-subscriber`, and a tree of `.rs` files. Rewritten for the Cyrius
  library, including the `dist/naad.cyr` consumption path, the `dist/naad.deps`
  sidecar, and the include order consumers need.
- **`CONTRIBUTING.md` demanded six `cargo` commands**, none of which can run, and
  mandated serde derives and `tracing` that the port removed. Rewritten around
  the real `cyrius` gates, the porting conventions, and the `flock` rule.
- **`SECURITY.md` declared the shipping line unsupported** — its only supported
  row was `0.1.x`, a version that was never git-tagged. Rewritten.
- **A durable rule in `CLAUDE.md` had gone false**: it stated `cyrius test` has
  *no auto-discovery* and each suite must be run by explicit path. Bare
  `cyrius test` discovers and runs all 38 suites, and `.github/workflows/ci.yml`
  has been relying on exactly that. The same false sentence was in
  `docs/development/state.md` and `docs/development/port-audit.md`.
- **`cyrius fmt <file>` rewrites the file IN PLACE** and prints nothing; the
  non-destructive form is `--check`. Documented as a hazard where the old text
  implied otherwise. (`cyrius audit`'s own hint text suggests a `-w` flag that
  does not exist.)
- **`docs/development/state.md` refreshed** — it is designated the authority on
  volatile state and had gone stale on the pin (`6.3.18`, a value that never
  matched even the pre-bump manifest), the assertion counts, and the dependency
  form, still describing hisab/goonj as path deps with goonj "commented out".
  Version numbers and counts now point at their source of truth instead of being
  inlined, since inlining is precisely what drifted.
- `cyrius.cyml`'s bundle comment said 443 top-level fns; measured **446**, all
  unique. The "collisions audited to zero" claim is **true** and is re-verified
  against hisab, goonj, sakshi, abaco and the stdlib — but it is fn-scoped and
  provably cannot see the `var` collision class, which is now noted there.
- `docs/development/roadmap.md` and `port-audit.md` refreshed; the shipped
  git+tag pinning ticked, historical counts stamped as-of rather than rewritten.

### Known — deliberately not fixed here

- **`ERR_INVALID_FREQUENCY` is defined twice**: naad's `src/error.cyr` (−1) and
  `lib/goonj.cyr` (−3), in one flat namespace. ⚠ Duplicate top-level `var`s draw
  **no diagnostic at all** from `cycc` or `cyrlint` — unlike duplicate `fn`s,
  which warn — so 2.1.1's "benign last-wins warning" does not apply to this
  class. It is inert today: goonj never returns the code, naad never calls
  goonj's error helpers, and every consumer includes goonj before naad. A
  `tests/bundle.tcyr` assertion now pins the resolved values so the collision
  becomes visible the moment either side's meaning is relied on. The durable fix
  — prefixing naad's block to `NAAD_ERR_*`, as hisab did with `HSB_ERR_*` — is a
  minor-release change, not a patch.
- **`fit_polynomial` still has a hard cliff at nx ≥ 5793**, inside
  `ganita_mat_least_squares`, which allocates an `nx × nx` matrix unchecked and
  has no way to report failure. This threshold is **unchanged by the bump**.
  Capping it in naad may be a deliberate divergence from the oracle rather than
  parity restoration, so it needs checking against hisab's Rust source first; it
  belongs upstream in ganita either way.
- `GANITA_MAT_MAX_ELEMS` is still derived from a 256 MiB `ALLOC_MAX` that is now
  2 GiB — 8× stale, but it errs conservative and changes no naad behaviour.
- 8 undocumented public fns keep `cyrius audit` at exit 1.
- CI runs the changelog gate → deps → build → test. fmt, lint, deny, bench and
  fuzz are still not wired in, and there is no gate for the symbol-collision
  class above.

## [2.1.1] - abaco↔naad namespace de-collision

Unblocks a downstream consumer (dhvani) from bundling **both** abaco and naad
in Cyrius's single flat distlib namespace. abaco's math-engine `dsp` module and
naad both exported bare `amplitude_to_db` / `db_to_amplitude`; concatenated into
one bundle they collided — a benign last-wins warning today (naad's own tests
never pull abaco's copies), a hard conflict once the Wave G distlib bundles
everything into one namespace. naad renames its two onto the `naad_` prefix it
already uses elsewhere (`naad_fdn_*`, `naad_analysis_*`), so the audio surface is
collision-free by construction. Top-level symbol intersection of `dist/naad.cyr`
× `dist/abaco.cyr`: **2 → 0**. Pure rename — no numerics change; the three
affected suites stay green at the same assertion counts (dsp_util 36, dynamics
14, eq 9).

A prior review confirmed naad should **not** take an abaco dependency to dedup
these: naad is the AGNOS audio-synthesis-primitives owner (abaco's `dsp` is the
interloper), the abaco bundle drags net/http/json/currency into a low-level audio
lib, and abaco's DSP formulas diverge from the `rust-old/` oracle (windows use
`N-1` vs naad's `N`; `time_constant` returns the raw pole vs naad's `1-exp` EMA
coeff) — so any swap would silently break parity. De-collision, not adoption, is
the correct resolution.

### Changed

- **dsp_util** — `amplitude_to_db` → `naad_amplitude_to_db`, `db_to_amplitude` →
  `naad_db_to_amplitude`. Internal callers (`eq` de-esser, `dynamics`
  compressor/limiter/noise-gate) and their tests updated in lockstep.
  `db_to_amplitude_lut` is a **distinct** function and is left unchanged.

### Migration

- Downstream consumers calling naad's `amplitude_to_db` / `db_to_amplitude` must
  switch to the `naad_`-prefixed names. No other naad surface is affected. Only
  these two names collided with abaco; naad's remaining bare `dsp_util` names
  (`lerp`, `rms`, `peak`, `normalize`, `hard_limit`, `soft_clip_tanh`,
  `hermite_interpolate`, `crossfade_equal_power`, `apply_hann_window`,
  `apply_blackman_window`, `eval_polynomial`, `xorshift32*`) are a known Wave G
  cleanup, tracked separately — not part of this fix.

## [2.1.0] - Post-port audit: correctness, security & hot-path memory

First work-loop pass over the 2.0.0 Cyrius port. A deep multi-agent review
(6 module groups × 4 lenses: correctness, memory-safety/security, performance,
refactor) surfaced 13 candidate findings; adversarial verification against the
`rust-old/` oracle confirmed **9** (4 rejected). All 9 repaired. **475 parity
assertions green** across 37 suites; `cyrius deny` clean.

### Fixed

- **dsp_spectral (security)** — `fft_magnitudes` / `power_spectrum` on empty
  input fell through hisab `num_fft`'s `n<=1` early-return into a `half=1` scale
  loop that did an out-of-bounds `load64` on a zero-byte alloc with a `1/0=+inf`
  scale (the Rust oracle panics on the length-0 slice). Added an `n==0 → empty`
  guard.
- **dsp_spectral (correctness)** — `fit_polynomial` on a rank-deficient design
  (e.g. duplicate x values) divided by a ~0 QR diagonal and returned a vec of
  NaN/inf coefficients; the oracle returns `None`. Now detects non-finite
  coefficients and returns the empty vec.

### Performance

- **Eliminated per-sample heap allocations** on four hot buffer paths — under
  Cyrius's free-less bump allocator these leaked unboundedly across a render:
  - `filter` — SVF lowpass routes through a new alloc-free `#inline` core;
    `filter_svf_process_buffer_lowpass` allocates **0 bytes/sample** (was one
    `SvfOutput`/sample — verified: 100k calls → 0 bytes allocated).
  - `reverb` — `reverb_process_buffer` reuses one scratch `ReverbStereo`.
  - `oscillator` — `unison_fill_buffer_stereo` writes L/R directly into the
    output vecs (was one `UnisonStereo`/sample).
  - `acoustics` — `MatrixFdn` owns two 8-slot scratch vecs allocated once in
    its constructor (was two throwaway vecs/sample).
  - `panning::pan_mono` — gains computed in locals (dropped a redundant
    `PanGains` alloc/call).
  Numerics unchanged (each module's parity suite green at the same assertion
  count); single-sample benchmarks show no regression.
- Added `#inline` to `filter_svf_process_sample` to match the oracle.

### Changed

- `physical` — corrected the module header: the Moog-ladder state is kept at
  f64 (consistent with naad's port-wide f32→f64 widening) — a precision
  refinement over the oracle's per-sample f32 re-quantization, not a bit-exact
  match. Both flush denormals.

### Added

- Regression tests for the empty-input and singular-system guards
  (dsp_spectral 40 → 43 assertions).

## [2.0.0] - Cyrius port

Complete rewrite from Rust to **Cyrius**. naad's Rust line shipped through 1.2.5;
the language port is a major break, so it lands as 2.0.0. The 13,465-line Rust
source is frozen at `rust-old/` as the parity oracle — every Cyrius module is
cross-checked against it function-for-function. Per-module ledger in
[`docs/development/port-audit.md`](docs/development/port-audit.md).

### Changed

- **Language**: Rust → Cyrius (`.cyr`). Toolchain pinned via
  `cyrius.cyml [package].cyrius` (6.3.19). Build with `cyrius build src/main.cyr
  build/naad`; test a suite with `cyrius test tests/<mod>.tcyr`.
- **All 41 modules ported** (L0 → acoustics): error, dsp_util + the
  `dsp_spectral` block (FFT/STFT/chromagram/onset/pitch/B-spline/poly-fit),
  oscillators (core/unison/sub/sync), filters, envelopes, wavetables (+ cubic
  B-spline morph), modulation, effects, dynamics, EQ, reverb, noise, panning,
  smoothing, tuning, voice, mod-matrix, delay, and the synthesis engines
  (subtractive, FM, additive, formant, granular, physical, vocoder, drum), plus
  9 acoustics wrappers over goonj (room, convolution, binaural, FDN, analysis,
  ambisonics, directivity, coupled + `material_by_name`).
- **f32 → f64** throughout (hisab's `HVec3`/`HComplex` are f64-only; widening
  is forced and improves precision).
- **Error handling**: the `NaadError` enum → integer codes (`ERR_*`); validators
  return `ERR_NONE`/negative; `Result`/`Option` → code or sentinel returns
  (null `0`, `NaN`, `-1.0`). No unwinding — Cyrius has none by design.
- **Dependencies**: `hisab` (math/geometry, `num_fft`, `calc_bspline`,
  least-squares) and `goonj` (acoustics engine) consumed as Cyrius distlib
  bundles.
  > **Erratum, noted at 2.1.2**: least-squares is not hisab's. `fit_polynomial`
  > calls `ganita_mat_*`, which comes from the stdlib `ganita` leaf, not from
  > `lib/hisab.cyr` (which defines no `ganita_mat_*`). The line is left as
  > written because it is a record; the misattribution is corrected in
  > `cyrius.cyml` and matters because that is where 2.1.2's null-deref
  > originated. `smallvec`/`serde`/`thiserror`/`tracing`/`criterion` dropped
  (`SmallVec`/`Vec` → stdlib `vec`; no serde in Cyrius).

### Added

- **Parity test suites**: one `tests/<mod>.tcyr` per module, each Rust `#[test]`
  ported one-for-one (serde round-trips + Display-string tests dropped) —
  **463 assertions across 36 suites, all green**.
- **`dist/naad.cyr`** distlib bundle (39 modules, dependency-ordered, one flat
  namespace; cross-module symbol collisions audited to zero — 443 top-level
  fns). Validated by `tests/bundle.tcyr`; consumers supply stdlib + hisab +
  goonj.
- **Hot-path benchmarks** (`cyrius bench tests/hotpath.bcyr`): oscillator,
  state-variable filter, ADSR envelope, pink noise (~1.4 µs/sample, scalar
  reference; hosts own SIMD dispatch).
  > **Annotated at 2.1.2 — this figure is ~96 % timer floor, not naad.** It was
  > produced by the pre-6.5.19 `bench_run`, which wrapped a clock pair around
  > every iteration and subtracted nothing. On this host a clock read costs
  > ~1.34 µs, so all four operations reported ~1.4 µs regardless of their own
  > cost. Re-measured under the calibrated instrument they span **4.78×** —
  > 23 ns to 105 ns (see 2.1.2). The number is left as written because it is a
  > record of what was measured; it is not comparable to anything measured
  > after the 6.5.19 boundary.

### Removed

- serde derives + all serde round-trip tests (no serde in Cyrius).
- `smallvec`, `thiserror`, `tracing`, `criterion` dependencies.

## [1.2.5] - P3 — `SmallVec` for fixed-size collections

### Added

- **dep**: `smallvec = "1"` with `serde` + `const_generics` features. Serializes as a sequence (Vec-compatible JSON shape), so existing serialized data deserializes cleanly into the new field types.

### Changed

- **P3 — Stack-allocated bounded collections**:
  - `voice::VoiceManager.voices` → `SmallVec<[Voice; 16]>`. Typical poly-synth configurations (4–16 voices) stay stack-resident; larger pools (up to the 128 cap) spill to the heap.
  - `synth::fm::FmSynthEngine.operators` → `SmallVec<[FmOperator; 6]>`. With `MAX_OPERATORS = 6`, every supported configuration is fully inline — no heap traffic for FM voices.
  - `synth::vocoder::Vocoder.bands` → `SmallVec<[VocoderBand; 16]>`. Typical 8–16-band channel vocoders stay stack-resident.
  - `synth::eq::ParametricEq.bands` → `SmallVec<[EqBand; 16]>`. Covers parametric (4–10 bands) and graphic (10 bands) EQs without heap.
  - SmallVec serializes as `Vec<T>` does, so JSON / on-disk preset shapes are unchanged. Pure perf change at the API boundary.

## [1.2.4] - H4 — color-free Hadamard FDN reverb

### Added

- **H4 — `acoustics::fdn_reverb::MatrixFdn`**: From-scratch 8-line FDN with an 8×8 normalised Sylvester-Hadamard feedback matrix (orthogonal, energy-preserving) — the "color-free" reverb topology the roadmap calls out. Delay lengths are mutually-coprime primes spanning ~30–80 ms (29.7/37.1/41.3/43.7/53.9/59.1/67.3/73.7 ms) for high modal density and no metallic comb resonances. Per-line damping gains are computed from RT60 + delay length per Schroeder's `g = 10^(-3D/(RT60·SR))`. The Hadamard transform is hand-unrolled (±1 entries only — no matmul). Lives alongside the existing goonj-backed `FdnReverb` rather than replacing it; pick `MatrixFdn` when you want timbre that's a function of `target_rt60` alone, decoupled from any room-acoustic interpretation. Tests cover Hadamard self-orthogonality (H·H = 8·I), reverb-tail production, decay across ~RT60 (5× early/late energy ratio), DC stability under continuous white noise, dry passthrough, invalid-input rejection, and serde roundtrip with deferred-buffer reconstruction.

## [1.2.3] - H3 — DCT additive amplitude compression

### Added

- **H3 — `AdditiveSynth::compress_amplitudes_dct` / `restore_amplitudes_dct`**: DCT-II / IDCT pair for compressing the partial amplitude bank — smooth amplitude envelopes (1/n harmonic rolloff, formant peaks) concentrate most energy in low-order DCT coefficients, so storing just the first K (where K ≪ num_partials) gives lossy spectral compression suitable for preset transmission. `compress_amplitudes_dct(num_coeffs)` clamps `num_coeffs` to `[1, num_partials]`; `restore_amplitudes_dct(&coeffs)` zero-pads, runs IDCT, clamps each restored amplitude to `[0.0, 1.0]`, and re-applies the Nyquist filter (above-Nyquist partials stay silent). Uses `hisab::num::dct` / `idct`. Tests verify: full roundtrip preserves amplitudes within 1e-4, 4-of-16 truncation holds RMSE < 0.15 (75 % storage reduction at modest perceptual cost) but isn't exactly zero, `num_coeffs` clamping, length-validation rejection, and Nyquist invariant under restore. Behind the `synthesis` feature.

## [1.2.2] - H1 — RK4-integrated Moog-ladder filter

### Added

- **H1 — `synth::physical::MoogLadder`**: Classic Moog-ladder lowpass filter (4 cascaded one-pole stages with global feedback `k` for resonance and a `tanh` saturator at every stage) integrated per-sample via one `hisab::num::rk4` step over `1 / sample_rate`. Substantially more accurate than the trapezoidal / one-sample-Euler discretisations common in DSP-equation Moog implementations, especially at high resonance and near the cutoff. Internal state is `f64` for ODE-integrator stability; output is `f32` clamped to `[-2.0, 2.0]` as a defensive guard against rare divergence at extreme parameter combinations. Tests verify high-frequency attenuation (5 kHz < 0.5× of 200 kHz at a 1 kHz cutoff), resonant ringing (4× tail vs dry baseline), DC stability under self-oscillation parameters, parameter clamping, invalid-input rejection, and serde roundtrip. Behind the `synthesis` feature.

## [1.2.1] - H2 — B-spline wavetable morph

### Added

- **H2 — `dsp_util::bspline_eval_1d`**: Scalar wrapper around `hisab::calc::bspline_eval` — lifts `&[f32]` control points into `Vec3.x`, generates a clamped open uniform knot vector internally (so the first/last control points are interpolated at `t=0`/`t=1`), returns the spline value's `.x`. Tests verify endpoint interpolation, smoothness (no kinks across a 1024-sample sweep), and rejection of invalid inputs (`degree=0`, fewer than `degree+1` control points, `t` outside `[0, 1]`). Allocates per call; the doc string flags this for hot-path callers.
- **H2 — `MorphWavetable::next_sample_smooth`**: Cubic-B-spline (degree 3) version of `next_sample`. Sweeping the morph parameter through *all* tables produces a `C²`-continuous output curve instead of the kinked piecewise-linear blend. Falls back to the existing linear `next_sample` when fewer than 4 tables are present (cubic B-spline needs ≥4 control points). Tests confirm: fallback equals linear with 2 tables, smooth diverges from linear meaningfully across a buffer with 5 tables, all samples finite at `position` boundaries (0.0, 0.5, 1.0). `synthesis` feature gate.

## [1.2.0] - Post-1.0 buckets — first wave

Eight roadmap items shipped: two `P*` perf optimizations, three `G*` goonj
acoustics wrappers, three `H*` hisab integrations. The remaining `H1` /
`H2` / `H3` / `H4` / `P3` items are deferred — see roadmap for status.

### Added

- **H5 — Polynomial-fit utilities in `dsp_util`**: `fit_polynomial(xs, ys, degree)` wraps `hisab::num::least_squares_poly` with input validation, and `eval_polynomial(coeffs, x)` is the matching Horner's-method evaluator (f64 internally for stability, returns f32). Tests verify exact recovery of a known quadratic, Horner correctness on a hand-computed polynomial, rejection of mismatched/undersized inputs, and a worked tanh-fit example showing a 5th-order polynomial stays within 0.015 of `tanh(x)` over `[-1.5, 1.5]` — the soft-clip sweet spot, suitable for replacing per-sample `tanh()` calls in distortion paths. Behind the `synthesis` feature.

- **H8 — Spectral analysis suite in `dsp_util`**: Three new primitives for offline analysis (svara composition tools, dhvani spectral metering):
  - `SpectralWindow` enum (`Rectangular` / `Hann` / `Hamming` / `Blackman`) with an in-place `apply` method.
  - `stft_magnitudes(signal, window_size, hop_size, window)` — short-time Fourier transform returning per-frame magnitude vectors. Tests verify dimensions match the spec, sine peaks land in the right bin (within 1.5 bins), and invalid inputs (non-power-of-two window, hop=0, undersized signal) return empty.
  - `chromagram(stft_frames, window_size, sample_rate)` — folds linear-frequency bins into the 12 chromatic pitch classes (A0–C8 range only). Test confirms a 440 Hz sine peaks at pitch class A (index 9) on every frame.
  - `detect_onsets(stft_frames, hop_size, sample_rate, threshold_factor)` — spectral-flux onset detector with running-median adaptive thresholding. Test confirms a 50ms burst at t=0.25s is detected within ±50ms.
  - All three behind the `synthesis` feature.

- **H6 — `envelope::CatmullRomEnvelope`**: Smooth-curve envelope generator that interpolates user-placed `EnvelopePoint` control points with Catmull-Rom cubics via `hisab::calc::catmull_rom`. C¹-continuous (no kinks at control points), unlike `MultiStageEnvelope`'s linear segments. Suited to organic/vocal amplitude shapes that linear ADSRs can't capture without dozens of segments. Phantom-endpoint clamping prevents overshoot before t=0 and after the last point. Tests verify the curve passes through control values, has no sharp deltas (max sample-to-sample |Δ| < 0.01 across a 100ms 0→1 rise), and rejects fewer-than-2 points / non-monotone times. Behind the `synthesis` feature.

- **H7 — `dsp_util::detect_pitch_autocorr`**: Autocorrelation-based pitch detection with sub-sample peak refinement via Newton-Raphson (`hisab::num::newton_raphson`) on a Catmull-Rom cubic interpolant through the four samples around the discrete peak. Falls back to closed-form parabolic interpolation as the NR initial guess. Tests confirm <5 cents accuracy on 220 Hz, 440 Hz, 880 Hz, 1100 Hz sines, *and* on 437.3 Hz (non-integer-period — the NR refinement is what keeps this within 5 cents). Rejects noise (<30 % autocorr-peak/r0 ratio), short buffers, and degenerate min/max ranges. Behind the `synthesis` feature.

- **G1 — `acoustics::analysis::suggest_absorption`**: Wraps `goonj::analysis::suggest_absorption_placement` to return per-wall RT60-sensitivity advice for tuning a virtual mix room toward a target reverb time. New `WallAbsorptionAdvice` struct (naad-side mirror of goonj's type, so future goonj API moves don't break naad's surface).
- **G2 — `acoustics::coupled` module**: New module wrapping `goonj::coupled::coupled_room_decay`. `CoupledRoomConfig` (two `RoomReverbConfig`s + a `CoupledPortal`) → `CoupledDecayResult` with double-slope RT60 (early/late), early-component amplitude, and coupling strength. Models live-room/control-room pairs, halls with reverberant side chapels, etc.
- **G3 — `acoustics::directivity` module**: New module exposing `SourceDirectivity` (omni / cardioid family / figure-8) for monitor-placement and spatial-synthesis use cases. Wraps `goonj::directivity::DirectivityPattern`'s closed-form variants; tabulated balloon data deliberately not exposed (the no-IO contract). Provides both 3D `gain(direction, front)` and 2D `gain_polar(theta)` evaluators with a roundtrip test verifying they agree on axisymmetric inputs.

- **P1 — `dsp_util::db_to_amplitude_lut`**: 256-entry LazyLock LUT covering `[-80, +20] dB` with linear interpolation, replacing per-sample `powf` in dynamics gain stages. Tests verify <0.5% error vs the `powf` reference across the table range and correct edge clamping. `Compressor::process_sample` now uses it.
- **P2 — Compressor knee specialization**: Hot path no longer runs the soft-knee branch when `knee_db == 0.0` (the common case). `compute_gain_db` splits into `compute_gain_db_hard` (no knee math) and `compute_gain_db_soft` (full version); `process_sample` dispatches once per sample instead of running the conditional inside the gain calc.
- **`compressor_1024` benchmark improved 15.7 µs → 14.6 µs (−7.0%, p<0.05)**. Measured under `cargo bench`, criterion 0.8.

## [1.1.0] - Project Organization & Cleanup

All 17 1.1.0 roadmap items (O1–O17) shipped. No new features — refactor,
encapsulation, dedup, tests, and docs only. Several breaking changes for
direct field readers / type importers; see individual entries below.

### Added

- **O1 — `oscillator.rs` (949 LOC) split into `oscillator/{mod,core,sub,sync,unison}.rs`**: Largest single source file in the crate is now five files; the heaviest is `core.rs` at 403 LOC (Waveform + Oscillator + polyblep + the stateless waveform helper). Tests follow the code into per-submodule `mod tests`. Public API is preserved via re-exports in `oscillator/mod.rs` — external callers continue to use `naad::oscillator::Oscillator` etc. with no change. `HardSync` now uses the public `Oscillator::phase()` accessor instead of the private field (it crossed a module boundary). The unison-oscillator initial-phase RNG also moved to the canonical `dsp_util::xorshift32_unit_f32` (a 5th call site that O2 missed).

- **O3 — `modulation::FmSynth` → `modulation::FmModulator`**: Disambiguates the simple two-operator FM primitive in `modulation` from the multi-operator [`synth::fm::FmSynthEngine`]. **Breaking**: imports/usages must update the type name. Doc comment now also points readers at `FmSynthEngine` for serial/stack/parallel topologies.
- **O4 — `dynamics::EnvelopeDetector` → `dynamics::LevelDetector`**: Disambiguates from the unrelated [`envelope::EnvelopeState`] in the ADSR module — both prefixes shared "Envelope" for entirely different concepts. **Breaking** for direct importers; the type is also re-used internally by `Compressor` and `NoiseGate`, but those are unaffected.

- **O2 — `dsp_util::xorshift32` (+ `_signed_f32`/`_unit_f32` wrappers)**: Single canonical xorshift32 step (with zero-state guard) replaces 6 inline copies across `noise`, `synth::granular`, `synth::physical`, and `synth::drum` (kick + snare). Callers keep their `noise_state: u32` field types — only the algorithm is centralized — so serde formats are unchanged. New unit tests cover the zero-state guard, determinism, and output-range invariants.

- **O9 — `ConvolutionReverb::process_block` reuses scratch buffers**: Three `Vec<Complex>` (zero-padded IR, zero-padded input, pointwise product) were allocated and dropped on every block. They now live as `#[serde(skip)]` fields on the struct and are reset via `clear() + reserve() + extend() + resize()` per call — capacity grows to fit the largest seen `fft_len` then stays put. `rebuild_from_ir` clears them so a smaller IR doesn't pay for a previous larger one's footprint. (Not bench-validated — no `convolution_reverb` bench exists yet; structural change only.)
- **O10 — `GranularEngine.grains` Vec → `[Grain; 64]`**: Fixed-size pool replaces heap Vec. `#[serde(skip)]` with `default_grain_pool` ctor — serde 1.x doesn't auto-derive arrays > 32, and `source` is already skipped, so reconstructing the grain pool to "all inactive" matches existing serde behavior (active grains can't play out of a non-existent source anyway).
- **O11 — `FormantFilter` Vec → `[BiquadFilter; 3]`**: Storage is now stack-allocated. Constructor enforces exactly 3 formants (returns `InvalidParameter` otherwise) instead of accepting any slice and silently misbehaving downstream. **Soft breaking change**: callers passing a slice with a non-3 length now get an error instead of an underfilled filter bank.

- **O5 + O6 — API Conventions section in `lib.rs`**: Documents the encapsulation rule (stateful types → private fields + setters; pure value-bag parameter structs → `pub` fields with stated semantics) and the constructor return-type rule (validating constructors return `Result`; clamping constructors are infallible; index-based mutators return `Option`/`Result`). Gives consumers an explicit map of why some types take `&mut self` setters and others let them poke fields directly.
- **O7 — `FmSynthEngine::set_operator_freq` / `set_operator_level` now return `Option<()>`**: They previously ignored bad indices silently; the return type now documents the failure mode so callers building algorithms dynamically can detect mistakes. **Breaking change** — callers discarding the result must add `let _ =` (Rust's `Option<()>` does not raise `unused_must_use`, but the source signature has changed).
- **O8 — Dynamics encapsulation**:
  - `Compressor.ratio` is now private with `ratio()` / `set_ratio()` accessors that re-apply the `>= 1.0` clamp. `threshold_db`, `knee_db`, `makeup_db` remain `pub` (direct-read parameter fields) and are documented as such.
  - `Limiter.ceiling_db` and `Limiter.release` are now private. They previously shadowed values inside the internal `Compressor` and modifying them did **nothing** at runtime. New `ceiling_db()` accessor and `set_ceiling_db()` mutator that propagates to the gain stage; `release()` accessor.
  - `NoiseGate.threshold_db` documented (no behavior change — `pub` is correct here).
  - **Breaking change** for consumers reading/writing `Compressor.ratio`, `Limiter.ceiling_db`, or `Limiter.release` as fields.

- **O12 — granular pitch-shift test**: `test_pitch_shift_changes_output_frequency` renders a 200 Hz sine source at `pitch_shift=1.0` vs `2.0` and asserts the zero-crossing density rises by ≥1.5×, confirming the playback rate actually retunes the output.
- **O13 — granular spray variation test**: `test_spray_produces_position_variance` runs the engine with `spray=0` and `spray=50ms` over a ramp source and asserts the rendered buffers diverge meaningfully (total |diff| > 1.0). Replaces the previous finiteness-only check with a real behavioral assertion.
- **O14 — dynamics edge cases**: `test_compressor_ratio_one_is_unity` (1:1 above threshold = no reduction), `test_noise_gate_hold_timer_keeps_gate_open` (gate stays open during hold window after env drops below threshold, then closes), `test_limiter_ceiling_exact_match_passes_through` (signal at ceiling passes unchanged, signal above is reduced) — covers boundary branches that previous tests missed.
- **O15 — granular serde functional test**: `test_serde_functional_reload` proves a deserialized engine is silent without a source, and that reloading a source post-deser produces audible output using the preserved configuration. Strengthens the field-only roundtrip check.
- **O16 — struct-level docs**: Expanded terse one-liners on `Grain`, `EqBand`, `VocoderBand`, `Partial` to describe their role in the parent type, field semantics (e.g. `Partial.phase` units), and lifecycle (e.g. engine-owned vs. user-constructed).
- **O17 — `#[must_use]` policy**: All sample-returning methods (`next_sample`, `next_value`, `next_sample_stereo`, `process_sample`, `process_sample_lowpass`, `process`) now carry `#[must_use]` — discarding a DSP sample is almost always a bug. 30 sites annotated across `delay`, `dynamics`, `effects`, `envelope`, `eq`, `filter`, `modulation`, `noise`, `oscillator`, `reverb`, `wavetable`, `synth::vocoder`, `acoustics::{binaural,convolution,fdn_reverb,room}`. Tests that intentionally advance state without consuming output now use `let _ = …`.

### Changed

- **Deps**: criterion 0.5 → 0.8 (dev-dep). Swapped `criterion::black_box` for `std::hint::black_box` in `benches/benchmarks.rs` (deprecated upstream).
- **deny.toml**: removed 4 unused license allowances (BSD-3-Clause, deprecated GPL-3.0, ISC, Unicode-DFS-2016) — only MIT, Apache-2.0, GPL-3.0-only, Unicode-3.0 are encountered in the dep tree.

## [1.0.0] - Phase 6: Integration Validation + Stable API

### Added

- **API stability audit** — all public struct fields with constructor validation now encapsulated: Wavetable, WavetableOscillator, MorphWavetable, CombFilter, AllpassDelay, FmSynth, RingModulator, NoiseGenerator, VoiceManager, Voice. Accessor methods added throughout.
- **FFT convolution** — `ConvolutionReverb::process_block()` uses overlap-save via `hisab::num::fft()` for O(N log N) per block (vs O(N) per sample in `process_sample`)
- **dhvani smoke test example** — `examples/dhvani_smoke_test.rs` demonstrates full synthesis chain: voice manager → unison oscillator → SVF filter → envelope → mod matrix → reverb → compressor → EQ → stereo panning
- **Shruti migration guide** — `docs/development/shruti-migration.md` maps all shruti-instruments types to naad equivalents with migration steps

### Changed

- VERSION bumped to 1.0.0 — stable API
- All public fields on stateful types now private with validated accessors
- `Voice::active` and `Voice::age` now private (use `is_active()`, `age()`)
- `VoiceManager::voices` now `pub(crate)` (use `voices()`, `voice_mut()`)

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
