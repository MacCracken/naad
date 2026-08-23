# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.1] - Accessor coverage: from 182 untested public functions to zero

The roadmap's largest remaining gate, closed. **Every one of naad's 440 public
functions is now referenced by a test** — the count was 182 untested when this
release started. Suite: **579 → 2340 assertions** across 40 suites.

No source behaviour changed. The only `src/` edit is a comment (see Documented
below). This is a test-only release.

### Why this mattered

The untested set was not random. Five of the eight functions that had to be
documented in 2.1.3 were in it — the same corners were missing both docs and
tests — and the 2.1.3 sweep found 16 defects in code the 505-assertion parity
suite passed clean. Ported suites inherit the oracle's *positive* cases, and
Rust's types made most negative cases unwritable, so whole families (all four FM
algorithms, every `*_process_buffer`, the four voice-steal modes, the ADSR state
machine) had zero assertions behind them.

⚠ **A vacuous test is worse than no test**, because it makes the gap invisible.
This repo has shipped one: an RT60 assertion compared through `f64_to`, which
truncates, so it held for every value in (−1.0, 1.0) and asserted nothing for
three releases. Guarding against a repeat was the design constraint for this
whole pass, not an afterthought.

### How the coverage was made falsifiable

- **Expectations are derived from the oracle, not observed from naad.** The
  xorshift32 streams, compressor knee gains, allpass/comb impulse responses,
  ADSR stage arithmetic and Audio-EQ-Cookbook endpoint gains were each
  re-derived independently from `rust-old/` and matched before being written
  down. Values transcribed from what naad happened to print would have frozen
  any existing bug in as "correct" — the worst possible outcome for a coverage
  pass.
- **Fixtures are chosen to make values exact.** 11025 Hz at 44.1 kHz gives a
  phase step of exactly 0.25; 10 Hz sample rates give 10-sample envelope stages;
  dyadic feedback coefficients and 4-sample tables keep every intermediate
  representable. That is what lets **1434** of the new assertions be raw-bit
  `assert_eq` rather than tolerance checks.
- **Buffer forms are diffed against per-sample forms** bit-for-bit, with
  sentinel fills proving the buffer was written and canary vecs proving it was
  not overrun — never "the call returned 0".
- **~40 assertions exist solely to prove another assertion is not `0 == 0`**
  ("reference produced real signal", "the compared output is not all zeros").
- **Setter round-trips use non-default values** and are each followed by a
  behavioural consequence — the coefficient the process path actually uses, not
  just the field read back.
- **State machines assert transitions**, with exact stage lengths
  (442 / 4411 / 13231 calls for 0.01 s / 0.1 s / 0.3 s at 44.1 kHz), which is
  what proves exactly one envelope pump per sample.

### Audited

An independent pass swept every added assertion for the known failure modes.
Results: **0** comparisons through `f64_to`; **0** tautologies; **0**
buffer-by-return-code assertions; all **155** newly declared IEEE-754 hex
constants decoded and checked against their decimal comments (155/155 correct);
tightest-to-loosest tolerances 1e-15 → 1e-4 with three deliberate 0.01 ordering
checks and none wide enough to admit a plausible wrong value.

**One genuine vacuity found and fixed**: a flag-accumulator loop in
`tests/acoustics_analysis.tcyr` compared field-by-field inside
`while (i < vec_len(goonj_result))` and would have held vacuously had goonj
returned an empty vec — the preceding length-equality assertion does not prevent
`0 == 0`. Now guarded by `assert_gt(vec_len(...), 0, ...)`. The other five
non-literal-bound loops were checked and are already guarded.

**One `naad_is_finite`-only assertion retained** (binaural with an emptied IR),
where "does not emit NaN" genuinely is the contract.

### Documented — a divergence found by writing the tests

- **`granular_rem_euclid` returns `+0.0` where Rust returns `−0.0`** for a
  negative exact multiple. Rust computes `r = a % b`, which is `−0.0` for
  `−3 % 3`, and `−0.0 < 0.0` is false so it returns `−0.0` unchanged; the
  trunc-and-correct form produces `+0.0`. **Inert and deliberately not
  corrected**: the result feeds `f64_floor` then `f64_to` (both zeros convert to
  0) and a `frac = pos − floor(pos)` that is `+0.0` either way. Now a recorded,
  tested contract instead of an accident. The only `src/` change in this release.

Three further findings are the standing **f32 → f64 port convention**
(`docs/development/port-audit.md`) rather than new divergences, but are worth
stating since the new tests pin f64 values: the noise generators' normalisation
runs in f64 where Rust used f32 (raw u32 draws are identical; the ~1e-7
difference compounds through pink's accumulator and brown's integrator); the
Moog ladder keeps full f64 RK4 state where Rust narrowed through f32 every
sample, so trajectories diverge at high resonance; and `f64_pow` has no exact
integral-exponent path, so `2^(n/12)` lands at 55.000000000000014 rather than
55. Nobody should read the new bit patterns as oracle-derived where f32 was
involved.

### Also recorded

Two **port-only contracts** are now frozen in the suite, correctly labelled and
with no oracle counterpart because the Rust enums are exhaustive:
`filter_biquad_with_gain` rejecting `filter_type > NAAD_FILTER_PEAK`, and
`osc_stateless_waveform_sample` returning `0.0` for out-of-range waveform ids.

`fm_engine_set_algorithm` validates nothing — any unknown integer silently means
CUSTOM routing rather than an error, a failure mode Rust's exhaustive `match`
did not have. Deliberately **not** asserted, because pinning it would bless
behaviour that may deserve an `NAAD_ERR_INVALID_PARAMETER` instead. Flagged for
a later decision.

### Known — still open

- **12 internal helpers** (`_`-prefixed) remain unreferenced by tests. They are
  exercised through their public callers; testing them directly would pin
  implementation detail rather than contract.
- **Consumer-green (dhvani / svara)**, which still needs the coordinated refresh
  of the five vendored `lib/naad.cyr` copies that 2.1.3 and 2.2.0 require.
- **Retiring `rust-old/`** stays gated on consumer-green.
- The **ganita** `mat_least_squares` defect is now filed upstream with a verified
  SIGSEGV repro. naad is not exposed to it after 2.2.0.

## [2.2.0] - The namespace wave, and parity restored on `fit_polynomial`

The deferred minor-release work from 2.1.3: everything that was real but not
patch-safe. Three breaking renames, one algorithm replacement that retires an
ADR, two deletions, six new public functions, and the CI gates that were
available but not enforced.

**`dist/naad.cyr` now collides with nothing.** Measured against **all 126
sibling bundles** on this machine plus the pinned stdlib: zero shared top-level
symbols, `fn`/`var`/`const`/`struct` alike. Before this release there were five.

Suite: **40 suites / 579 assertions**, `cyrius audit` exits 0, and the fuzz
harness runs **1273 adversarial checks**.

### Changed — BREAKING

**Error constants** were renamed in 2.1.3; **these are the rest of the
namespace wave.** All values are unchanged — only the names move.

- **`FILTER_*` → `NAAD_FILTER_*`** (all 8: `LOWPASS`, `HIGHPASS`, `BANDPASS`,
  `NOTCH`, `ALLPASS`, `LOWSHELF`, `HIGHSHELF`, `PEAK`).
  `nidhi` defines `FILTER_LOWPASS`..`FILTER_NOTCH` as 0..3 — **identical values**
  to naad's — and is co-linked with naad inside dhvani today. Nothing
  misbehaved, which is exactly the problem: a flat-namespace collision between
  two enums that happen to agree is invisible until one side renumbers. All
  eight are renamed rather than the four that collide, because a half-prefixed
  enum is worse than either alternative.
- **`VOICE_*` → `NAAD_VOICE_*`** (`NONE`, `DEFAULT_BRIGHTNESS`, `AGE_MAX`).
  `garjan` defines `VOICE_NONE = -1`, again identical.
- **Six bare function names → `naad_*`**: `lerp`, `rms`, `peak`, `normalize`,
  `chromagram`, `crossfade_equal_power`. Zero collisions today — these are
  forward risk, and `lerp` in particular is the single most likely name for any
  future geometry or animation sibling. The 2.1.1 CHANGELOG logged them as a
  known deferred cleanup; this closes it.

**Deliberately NOT renamed**, and the reasoning, so nobody re-opens it: the
compound-prefixed families (`WAVEFORM_*`, `NOISE_*`, `DISTORTION_*`,
`MOD_DEST_*`, `TUNING_*`, `GRAIN_WINDOW_*`, `POLY_MODE_*`, `STEAL_MODE_*`,
`MODULATION_LFO_*`, `PAN_LAW_*`, `FM_ALGO_*`) and the ~400 already-prefixed
function families. They are effectively namespaced by their own compound
prefix, measured at zero collision risk across the whole ecosystem, and
renaming them would add verbosity to the public API and ripple through five
vendored copies for nothing.

### Changed — `fit_polynomial` now ports the oracle's thin QR

`fit_polynomial` called `ganita_mat_least_squares`, which materialises a **full
`m × m` orthogonal Q** that polynomial least-squares does not need, never checks
its own allocation, and therefore faulted from 5793 samples upward. 2.1.3 capped
the input and recorded the cap as a knowing divergence in ADR-0001.

2.2.0 ports hisab 1.4.0's `qr_decompose` + `least_squares_poly` directly
(modified Gram-Schmidt over the Vandermonde, then back-substitution, with `R`
stored `r[col][row]` exactly as the oracle stores it). **Q is `m × n`.** For a
degree-2 fit over 100 000 samples that is **2.4 MB instead of 80 TB**.

- The cap is gone; large inputs succeed as they do in Rust. `tests/hardening.tcyr`
  asserts a 20 000-sample fit.
- naad no longer references `ganita_mat_*` at all, so ADR-0001's residual
  allocation-failure hole — which the cap did *not* close — is gone with it.
- **[ADR-0001](docs/adr/0001-fit-polynomial-sample-cap.md) is superseded by
  [ADR-0002](docs/adr/0002-port-thin-qr-in-tree.md).**

⚠ **This changes floating-point rounding for every input that already worked.**
Gram-Schmidt over the Vandermonde is not the same arithmetic as ganita's path,
so coefficients move in the last few ulps. Nothing in naad asserts bit-exact
coefficients and every tolerance-based assertion passes unchanged — but a
consumer comparing against stored coefficients will see a difference. This is
the reason it is a minor release.

One divergence is **retained and now documented precisely**: a non-finite
*input* yields a NaN norm, which passes the oracle's `< EPSILON` singularity
test, so Rust returns `Ok` with NaN coefficients. naad returns the empty vec —
the same `None` channel, rather than poisoned coefficients a caller has no
obvious way to notice. It predates this release (2.1.0). The *singular* case —
duplicate x values — is now exact parity via the ported norm check.

### Changed — `tuning_note_name` domain guard

The oracle takes `note: u8`, so 0..255 is its entire domain and it needs no
guard. The port widened the parameter to a signed `i64` and formatted a nonsense
octave for values Rust could never receive. Notes outside 0..127 now return `""`.
`tuning_note_name_str` gains the same treatment: a negative pitch class used to
fall through every equality test and silently return `"B"`.

2.1.3 widened the output buffer from 16 to 32 bytes for the same underlying
reason; that stays as defence in depth, but with this guard the real bound is
5 bytes.

### Added

- **Zero-alloc siblings** for the three modules that had only an allocating
  per-sample form and no escape hatch. Same `_into` / `_buffer` split as the
  existing `reverb_process_core` / `reverb_process_buffer`:
  `panning_pan_mono_into` · `panning_pan_buffer` ·
  `naad_ambisonics_encode_sample_into` · `naad_ambisonics_encode_buffer` ·
  `naad_binaural_process_sample_into` · `naad_binaural_process_buffer`.
  The one-shot forms are unchanged and still allocate — that is their contract.
  `tests/allocbudget.tcyr` pins the new forms at zero per-sample bytes, with a
  control asserting the one-shot form *does* still allocate, so the budget is
  measuring something real rather than an inlined no-op.
- **A real fuzz harness.** `tests/naad.fcyr` was a stub that called one function
  with a fixed string and printed "ok". It now drives the public boundary with
  the values that actually break Cyrius ports — `INT64_MIN`, NaN, ±inf,
  subnormals, `f64::MAX`, the 709/710 `f64_exp` boundary, and non-power-of-two
  lengths — across 1273 checks.
  ⚠ **The first version of this harness was worthless and measuring caught it.**
  Driving the enum id and the numeric parameters from one loop index correlated
  them, so an out-of-range id only ever paired with an already-invalid sample
  rate and was rejected before the id guard was reached: reverting all three id
  guards produced a *clean run*. The sweeps are now separated — ids swept at
  known-valid parameters, values swept at a known-valid id — and reverting any
  of the three guards now fails the harness.
- **CI enforces the quality gates it previously only had available**:
  `cyrius audit` (fmt · lint · docs · tests · bench), `cyrius deny`, and
  `cyrius fuzz`. Possible because `cyrius audit` started exiting 0 in 2.1.3.

### Removed — BREAKING

- **`white_noise_sample`** (`src/dsp_util.cyr`). Byte-identical to
  `noise_white_noise_sample` and **port-invented**: the oracle has exactly one
  such function, in `noise.rs`, which the port already placed in `src/noise.cyr`.
  It had no caller, and it was the copy occupying the dangerous bare name.
  Use `noise_white_noise_sample`.
- **`U32_MAXF`** (`src/dsp_util.cyr`). Self-labelled `# not used; kept for
  reference`, with zero references anywhere.

### Migration

Mechanical, and all of it is a rename. Values and behaviour are unchanged except
where noted above.

```
ERR_*                    -> NAAD_ERR_*                    (2.1.3)
FILTER_*                 -> NAAD_FILTER_*
VOICE_*                  -> NAAD_VOICE_*
lerp                     -> naad_lerp
rms                      -> naad_rms
peak                     -> naad_peak
normalize                -> naad_normalize
chromagram               -> naad_chromagram
crossfade_equal_power    -> naad_crossfade_equal_power
white_noise_sample       -> noise_white_noise_sample      (removed)
U32_MAXF                 -> (removed, was unused)
```

Two behavioural changes to check for:

- `tuning_note_name` / `tuning_note_name_str` return `""` outside their MIDI
  ranges instead of a formatted nonsense value.
- `fit_polynomial` coefficients move in the last few ulps, and inputs above
  ~5 800 samples now **succeed** where 2.1.3 returned the empty vec.

### Known — still open

- **108 public fns have no caller outside their own definition** — almost all
  accessors, correctly public but **untested**. This is the largest remaining
  gap and it is a coverage problem, not a code problem.
- **Upstream on ganita**: `ganita_mat_least_squares` still forms a square Q and
  still has no failure return. naad is no longer exposed to it, but the bug is
  real and belongs upstream.
- **Consumer-green (dhvani / svara)** — and note that this release's renames
  require a coordinated refresh of the five vendored `lib/naad.cyr` copies
  (dhvani, garjan, ghurni, nidhi, prani).

## [2.1.3] - P-1 hardening: the negative-input sweep

A P-1 audit / refactor / hardening / security sweep, plus the `ERR_*` →
`NAAD_ERR_*` de-collision the roadmap had been carrying since 2.1.1.

**The 505-assertion parity suite passed every one of these defects.** It had to:
not one of its assertions passed an out-of-range enum id, a negative count, a
NaN, or a non-power-of-two buffer length — which is the exact shape of all 22
findings. Four lenses swept `src/` and every finding was adversarially
re-derived before it was believed. **16 code defects fixed, 3 of which
reproduce as SIGSEGV**, and every fix is pinned by a test verified to *fail*
without it: each guard was reverted one at a time and the suite re-run, 19 times.

The defects are not scattered. They cluster into two mechanical classes, both
predictable from the port's semantics and now enumerated:

1. **Rust `usize`/enum guarantees erased by Cyrius's signed `i64`.** Where the
   oracle types a parameter `usize`, `u32` or an enum, a bad value is
   *unrepresentable* in Rust — so the Rust body validates nothing, and a
   transliterated guard is only **half a bound**.
2. **`f64_to` truncating where Rust's `as` casts SATURATE.** Rust's `as usize`
   maps NaN and negatives to 0; `f64_to` overflows to `INT64_MIN`, which is
   neither `> MAX` nor `== 0`, so every one-sided clamp written in the Rust
   idiom lets it straight through.

`cyrius audit` **exits 0 for the first time** — fmt, lint, docs, tests and bench
all clean. Suite: **40 suites / 557 assertions**.

### Changed — `ERR_*` → `NAAD_ERR_*` (BREAKING for consumers)

naad's six error constants were bare top-level `var`s sharing Cyrius's flat
distlib namespace with goonj's. Two agreed by value; `ERR_INVALID_FREQUENCY` did
**not** — naad `-1`, goonj `-3` — so which one a consumer got depended on
include order.

⚠ **Duplicate top-level `var`s emit NO diagnostic from `cycc` or `cyrlint`**,
unlike duplicate `fn`s. 2.1.1's "benign last-wins warning" does not apply to this
class, and the bundle's collision audit was fn-scoped and provably could not see
it. That is how this survived three releases.

- `ERR_NONE` → `NAAD_ERR_NONE`, `ERR_INVALID_FREQUENCY` →
  `NAAD_ERR_INVALID_FREQUENCY`, `ERR_INVALID_SAMPLE_RATE` →
  `NAAD_ERR_INVALID_SAMPLE_RATE`, `ERR_INVALID_PARAMETER` →
  `NAAD_ERR_INVALID_PARAMETER`, `ERR_BUFFER_OVERFLOW` →
  `NAAD_ERR_BUFFER_OVERFLOW`, `ERR_COMPUTATION` → `NAAD_ERR_COMPUTATION`.
  Values unchanged; 221 references updated in lockstep.
- The top-level symbol set of `dist/naad.cyr` (759 symbols) is now **disjoint
  from hisab, goonj, sakshi, abaco and the whole 6.5.35 stdlib** — measured in
  all five directions, `fn`/`var`/`const`/`struct` alike, not just `fn`.
- `tests/bundle.tcyr` now makes the assertion that was **impossible** before the
  rename: both libraries' frequency codes visible at once, each with its own
  value.

### Fixed — memory safety

- **`osc_new` accepted any integer as `waveform`** — the library's most-used
  constructor. Ids outside 0..7 left `noise_gen` null behind a **valid returned
  pointer**, and the dispatch chain's bare `else` dereferenced it on the first
  sample. The oracle matches an 8-variant enum exhaustively *and* carries an
  `else { 0.0 }` arm, so both the constructor guard and the null-safe fallback
  are parity-restoring. Reproduces as **SIGSEGV**. Also reached through
  `modulation_ring_new`, `modulation_lfo_from_waveform`, `subtractive_new`,
  `subtractive_set_osc2`, `hardsync_new` and `subosc_new`.
- **`bspline_eval_1d` had no `degree < 0` guard.** A degree `<= -2` drives the
  knot count to 0 while the interior count grows, so the knot loop writes past
  its buffer — or through a null once `alloc` sees a non-positive size. Oracle
  types `degree` as `usize`. Reproduces as **SIGSEGV**.
- **`fit_polynomial` faulted at `nx >= 5793`.** 2.1.2 null-checked the matrix it
  allocates itself, but `ganita_mat_least_squares` allocates an `nx × nx`
  orthogonal Q *internally* and never checks it. Reproduces as **SIGSEGV**.
  ⚠ The guard is a **deliberate divergence** — hisab 1.4.0 used a thin QR
  (`Q` is `nx × cols`) and returns coefficients at every `nx`, verified by
  reading the crate the oracle's `Cargo.lock` pins. Shipped with
  [ADR-0001](docs/adr/0001-fit-polynomial-sample-cap.md), which also records the
  residual `alloc`-failure path this does **not** close.
- **`wavetable_from_harmonics` guarded `size == 0` but not `size < 0`**, leaving
  an empty samples vec behind a valid pointer; the first read divided by zero
  (**SIGFPE**). It also passed `wavetable_morph_new`'s equality-only length
  check, so a MorphWavetable built from one faulted on its first sample.
- **`fm_engine_new` guarded `num_operators == 0` but not negative**, leaving the
  operator vec empty for a default algorithm that immediately indexes slot 0.
- **`tuning_table_custom` never validated the ratio COUNT.** The oracle's
  parameter is `[f32; 12]` — the length is a *type invariant*, which is why the
  Rust body validates only values. Erasing the array to a vec dropped it, and
  `tuning_note_to_freq` reads index 9 unconditionally.
- **`mod_matrix` source/destination ids were unvalidated** where the oracle used
  the `ModSource`/`ModDestination` enums; the scratch vecs hold exactly 8 slots.
  Note the deliberate asymmetry: `mod_matrix_set_source` returns an error code,
  while `mod_matrix_get_destination` returns an f64 and so reports an
  out-of-range id as `+0.0` — the honest answer for a destination that does not
  exist. A negative code is never smuggled through an f64 return.
- **`tuning_note_name` formatted into a u8-era 16-byte buffer.** The oracle takes
  `note: u8` (longest output `"C#10"`); the port widened the parameter to signed
  `i64`, where the integer formatter can emit 20 characters. Now 32 bytes.
  Honest note: this one ships **on inspection** — under the bump allocator the
  overflow currently lands on already-consumed scratch, so no runtime assertion
  can discriminate. The testable fix is a domain guard, which changes output for
  `note > 127` and is therefore not patch-safe.

### Fixed — wrong audio, no crash

- **`filter_biquad`'s coefficient chain had no `else` arm.** An out-of-range
  filter type left `a0` at `+0.0`, made `1/a0` `+Inf`, and every coefficient
  `NaN` — then latched NaN into `z1`/`z2` **permanently**, since
  `filter_biquad_reset` clears state but not coefficients. `eq_add_band`
  forwarded the bad type and returned success, contradicting its own doc. Both
  halves fixed: the constructor validates, and the chain closes with a unity
  pass-through so the public `BiquadFilter_set_filter_type` accessor cannot
  publish NaN either.
- **The LFO `shape` was never validated.** The dispatch chain has no default and
  fell through to the sample-and-hold slot — whose re-roll is itself gated on
  `shape == SAMPLE_AND_HOLD` — so an out-of-range shape emitted a **frozen DC
  value forever**. Every modulation destination got a constant bias instead of
  movement, with no error at construction or set time. Harder to notice than the
  NaN case: it produces plausible-looking, completely wrong audio.
  `modulation_lfo_set_shape` now returns an error code where it previously
  always returned 0. `modulation_lfo_set_mode` is deliberately **not** guarded —
  an unknown mode falls through to bipolar, which is the oracle's own default.

### Fixed — the `f64_to` truncation cluster

Five index sites clamped in int space *after* the cast, each written in the Rust
idiom and each inheriting a guarantee the cast no longer provides. All now
saturate at the cast: `wavetable_read_interpolated`,
`wavetable_morph_next_sample`, `wavetable_morph_next_sample_smooth`,
`granular_next_sample`, and `db_to_amplitude_lut` (made two-sided). Rust returns
a NaN sample and keeps running; naad aborted the process.

⚠ **`INT64_MIN % 1024 == 0`.** With a power-of-two buffer the pre-fix code lands
on index 0 and behaves correctly — and audio buffers are habitually
power-of-two. That is precisely why the parity suite never caught this, and why
the new tests deliberately use lengths of **100 and 1000**.

Two proposed fixes were **rejected as themselves-divergent**: adding
`naad_is_finite` rejection to `granular_set_position` / `wavetable_morph_set_morph`
(the oracle has no finite gate there and stores NaN happily), and returning
`table[0]` for a NaN in `db_to_amplitude_lut` (Rust computes
`table[0]*(1-NaN) + table[1]*NaN` = **NaN**, so the two-sided clamp is exact
parity). Saturation at the cast site only.

### Fixed — unbounded heap growth

`lib/alloc.cyr` is a **bump allocator with no individual free**, so a per-sample
allocation is a permanent leak, not churn. Rust drops these at scope exit — each
item below is a divergence created by the port's allocator, not by the
algorithm. All four now reuse struct-owned scratch. No numeric output changes.

| Path | Was | Rate at 48 kHz |
|---|---|---|
| `naad_convolution_process_block` | 3 × `fft_len × 16` **per block** | ~147 MB/s |
| `wavetable_morph_next_sample_smooth` | ~464 B/sample | ~22 MB/s |
| `unison_next_sample_stereo` | 304 B/sample | ~15 MB/s |
| `envelope_catmull_rom_next_value` | 120 B/sample | ~5.8 MB/s |

- The convolution fix also **null-checks each scratch allocation** and falls back
  to direct convolution — the leak had made its own null-deref reachable. The
  module header's claim that per-call allocation was "behaviourally identical"
  is corrected: identical numerically, false for resource behaviour, which was
  the entire point of the oracle's design.
- `bspline_eval_1d` is split into an allocating wrapper and a scratch-taking
  `_bspline_eval_1d_into` core, mirroring the existing
  `naad_fdn_hadamard8` / `_into` split.
- ⚠ **The smooth-morph path is NOT zero-alloc and must not be described as one.**
  About 120 B/sample of it lives inside `lib/hisab.cyr`'s `calc_bspline`, which
  naad may not patch. The new budget pins what naad controls.
- **Correction to the 2.1.0 record**: "oscillator hot path — 0 bytes/sample,
  verified" was true for `src/osc_core.cyr` and false for `src/osc_unison.cyr`'s
  stereo path.

### Added

- **`tests/hardening.tcyr`** (41 assertions) — every negative case above. Kept in
  its own file deliberately: most of these *abort the process* before the fix, so
  in a shared suite a pre-fix run dies partway through and prints a confusing
  partial result.
- **`tests/allocbudget.tcyr`** (11 assertions) — `alloc_used()` sandwiches over
  10 000-sample renders. This is the **first thing in the repo to pin the
  "0 bytes/sample" claim** that had been load-bearing in the docs since 2.1.0:
  `tests/hotpath.bcyr` benches four already-alloc-free functions and reports
  nanoseconds, never bytes, so four *other* hot paths regressed unnoticed.
  `alloc_used()` was referenced by nothing in this repo before now.
- **`scripts/symbol-collision-check.sh`** + a CI step — intersects
  `fn`/`var`/`const`/`struct` between `dist/naad.cyr` and every dependency
  bundle and the pinned stdlib snapshot. Deliberately **not** fn-scoped, since
  fn-scoping is what missed the `ERR_*` collision. Verified to fail when the
  2.1.2 collision is reintroduced.
- **A CI bundle-freshness gate** — `dist/naad.cyr` is a tracked artifact
  consumers link directly; if `src/` changes without regenerating it, the bundle
  on `main` is stale and `tests/bundle.tcyr` exercises last release's code.
- **[ADR-0001](docs/adr/0001-fit-polynomial-sample-cap.md)** — the first ADR in
  the repo; the index had claimed *"No ADRs yet"*.
- Doc comments for the 8 undocumented public fns. Seven sat *second* under a
  shared banner that already named them — none was genuinely undocumented, and
  they were the sole reason `cyrius audit` exited 1.

### Migration

Consumers must rename six constants: `ERR_NONE` → `NAAD_ERR_NONE`,
`ERR_INVALID_FREQUENCY` → `NAAD_ERR_INVALID_FREQUENCY`, `ERR_INVALID_SAMPLE_RATE`
→ `NAAD_ERR_INVALID_SAMPLE_RATE`, `ERR_INVALID_PARAMETER` →
`NAAD_ERR_INVALID_PARAMETER`, `ERR_BUFFER_OVERFLOW` → `NAAD_ERR_BUFFER_OVERFLOW`,
`ERR_COMPUTATION` → `NAAD_ERR_COMPUTATION`. Values are unchanged, so a consumer
comparing against literals is unaffected. `naad_is_err` is the recommended test
and is unchanged.

Constructors that previously accepted an out-of-range enum id now return a
negative `NAAD_ERR_INVALID_PARAMETER`. Any consumer relying on that acceptance
was building an object that would crash or emit NaN, so the failure surfaces
earlier and more legibly — but it does surface where it previously did not.

### Known — deliberately not in this release

Each is real; none is patch-safe. All are 2.2.0 candidates.

- **`FILTER_*` and `VOICE_NONE` are unprefixed** and collide by name with
  `nidhi` and `garjan`, both co-linked with naad in dhvani today. Currently
  harmless — every shared value agrees, and nidhi's ids sit inside naad's valid
  band — so this is forward risk, not a live defect. A breaking rename of 9
  public symbols rippling into five vendored copies.
- **Zero-alloc siblings** for `naad_ambisonics_encode_sample`,
  `naad_binaural_process_sample` and `panning_pan_mono`, which today have only
  the allocating per-sample form with no escape hatch. New public functions.
- **Thin-QR `fit_polynomial`** — restores full parity, removes the ADR-0001 cap
  and the 268 MB Q. Different rounding for every currently-working input.
- **`tuning_note_name` domain guard** — the parity-faithful fix, and the only
  testable one; changes output for notes above 127.
- **Prefixing the Tier-1 bare names** (`lerp`, `rms`, `peak`, `normalize`,
  `chromagram`, `crossfade_equal_power`). Zero collisions today.
- **Upstream on ganita**: give `ganita_mat_least_squares` a failure return, or
  switch it to thin QR so no `nx × nx` Q is ever formed.
- **108 public fns have no caller outside their own definition** — almost all
  accessors, correctly public but **untested**. Five of the eight undocumented
  fns were in that set: the same corners were missing both docs and tests.

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
