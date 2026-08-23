# naad — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

`VERSION` at the repo root is the source of truth for the current number. The
entries below are the release record.

**2.2.0** — the namespace wave, and parity restored on `fit_polynomial`.

Everything 2.1.3 identified as real but not patch-safe. Three breaking renames,
one algorithm replacement that retires an ADR, two deletions, six new public
functions, and the CI gates that had been available but unenforced.

**`dist/naad.cyr` now collides with nothing.** Measured against all 126 sibling
bundles on this machine plus the pinned stdlib snapshot: zero shared top-level
symbols, `fn`/`var`/`const`/`struct` alike. There were five before this release
(`FILTER_LOWPASS`..`FILTER_NOTCH` against nidhi, `VOICE_NONE` against garjan) —
all with *identical values*, which is precisely why nothing misbehaved and why
nothing would have until one side renumbered. Re-measure with
`./scripts/symbol-collision-check.sh` plus any sibling bundles as arguments.

`fit_polynomial` no longer calls `ganita_mat_least_squares`, which materialised
a full `m × m` orthogonal Q that polynomial least-squares does not need. It ports
hisab 1.4.0's thin QR directly (Q is `m × n`) — 2.4 MB instead of 80 TB for a
degree-2 fit over 100 000 samples. The ADR-0001 cap is gone, large inputs succeed
as they do in Rust, and ADR-0001's residual allocation-failure hole goes with it.
⚠ Coefficients move in the last few ulps for every input that already worked;
that is why it needed a minor release. [ADR-0001](../adr/0001-fit-polynomial-sample-cap.md)
is superseded by [ADR-0002](../adr/0002-port-thin-qr-in-tree.md).

`tests/naad.fcyr` became a real fuzz harness — it was a stub that called one
function with a fixed string. ⚠ **Its first version was worthless and only
measuring caught it**: driving the enum id and the numeric parameters from one
loop index correlated them, so an out-of-range id only ever paired with an
already-invalid sample rate and never reached the id guard. Reverting all three
2.1.3 id guards produced a clean run. The sweeps are now separated and reverting
any of them fails the harness. 1273 checks.

CI now enforces `cyrius audit`, `cyrius deny` and `cyrius fuzz` alongside the
changelog, symbol-collision and bundle-freshness gates — possible only because
`cyrius audit` started exiting 0 in 2.1.3.

Suite: **40 suites / 579 assertions, 0 failed.**

**2.1.3** — P-1 hardening sweep, and the `ERR_*` → `NAAD_ERR_*` de-collision
the roadmap had carried since 2.1.1.

Four lenses swept `src/` (memory safety, numeric robustness, resource
discipline, API hygiene); every finding was adversarially re-derived before it
was believed. **16 code defects fixed, 3 reproducing as SIGSEGV.** Each fix is
pinned by a test verified to *fail* without it — every guard was reverted one at
a time and the suite re-run, 19 times.

⚠ **The 505-assertion parity suite passed all 16.** It had to: none of its
assertions passed an out-of-range enum id, a negative count, a NaN, or a
non-power-of-two buffer length. That is the whole shape of the finding set, and
it is the structural gap this release closes — the code was in better shape than
the tests were.

Two mechanical classes account for nearly all of it, both predictable from the
port's semantics:

1. **Rust `usize`/enum guarantees erased by Cyrius's signed `i64`.** Where the
   oracle types a parameter `usize`, `u32` or an enum, a bad value is
   unrepresentable in Rust, so the Rust body validates nothing — and a guard
   transliterated from it is only **half a bound**.
2. **`f64_to` truncating where Rust's `as` SATURATES.** `as usize` maps NaN and
   negatives to 0; `f64_to` overflows to `INT64_MIN`, which is neither `> MAX`
   nor `== 0`, so one-sided clamps written in the Rust idiom let it through.
   ⚠ `INT64_MIN % 1024 == 0`, so power-of-two buffers mask it entirely — which
   is exactly why the suite was green.

Headline fixes: `osc_new` accepted any integer waveform and returned a valid
pointer with a null noise generator (the most-used constructor in the library);
`bspline_eval_1d` had no `degree < 0` guard; `fit_polynomial` faulted at
`nx >= 5793` inside ganita's internal `nx × nx` Q. Four hot paths were leaking
against a bump allocator with no free — convolution `process_block` at
~147 MB/s. `filter_biquad`'s unclosed coefficient chain produced a permanently
NaN filter, and an unvalidated LFO shape produced frozen DC: both wrong audio
with no crash and no error.

The `fit_polynomial` cap is a **deliberate divergence**, shipped with
[ADR-0001](../adr/0001-fit-polynomial-sample-cap.md) — the repo's first ADR.
hisab 1.4.0 used a thin QR and succeeds at every `nx`; verified by reading the
crate the oracle's `Cargo.lock` pins.

`cyrius audit` **exits 0 for the first time**: fmt, lint, docs, tests and bench
all clean. Suite: **40 suites / 557 assertions, 0 failed** (+52).

New guards against recurrence: `tests/hardening.tcyr` (41 negative-input
assertions, isolated because most abort the process pre-fix),
`tests/allocbudget.tcyr` (11 `alloc_used()` budgets — the first thing in the
repo to pin the "0 bytes/sample" claim the docs had carried since 2.1.0), and
`scripts/symbol-collision-check.sh` wired into CI alongside a bundle-freshness
gate.

**2.1.2** — toolchain + dependency bump, and the two divergences it exposed.
Four coordinated version moves: the Cyrius pin `6.3.19 → 6.5.35`, `hisab
2.6.7 → 2.11.2`, `goonj 2.0.0 → 2.0.4` (goonj 2.0.4 itself pins hisab 2.11.2,
so the two agree), and `sakshi 2.4.2 → 2.4.11` arriving transitively via hisab.
`cyrius deps` re-vendored `lib/`; every file there that the 6.5.35 snapshot
also ships is byte-identical to it (re-verify with
`for f in lib/*.cyr; do cmp -s "$f" ~/.cyrius/versions/$(grep -oP 'cyrius = "\K[^"]+' cyrius.cyml)/lib/$(basename "$f") || echo "$f"; done`
— only `lib/hisab.cyr` and `lib/goonj.cyr` are expected to print, since those
come from the git+tag deps, not the toolchain snapshot).

`dist/` was regenerated. `dist/naad.deps` had been **empty** — consumers were
never being told naad's stdlib requirements; it now carries all 15 stdlib
leaves the bundle needs (`grep -vc '^#' dist/naad.deps`).

Three code changes landed, each pinned by a test proven to discriminate:

- **Five acoustics guards widened** from `sample_rate == 0` to
  `sample_rate <= 0` (`src/acoustics_fdn.cyr:78`, `:228`,
  `src/acoustics_analysis.cyr:82`, `:126`, `src/acoustics_binaural.cyr:108`).
  The oracle types `sample_rate` as `u32`, so negatives are unrepresentable in
  Rust; the port widened it to signed `i64` and transliterated the guard
  verbatim. Worst case was an FDN reverb whose damping gains exceeded 1 and
  grew ~1000× per 48000 samples while the constructor reported success.
- **`fit_polynomial` null-check** in `src/dsp_spectral.cyr`: a `return` on a
  failed `ganita_mat_new`, plus a `degree < 0` guard. This is a real regression
  *introduced by this bump* — ganita 1.1.4 gave `ganita_mat_new` failure
  returns where it previously always handed back a usable header, and the null
  reached `ganita_mat_least_squares`, which dereferences it immediately. Proven
  by execution: removing both guards makes `tests/dsp_spectral.tcyr` exit 139
  (SIGSEGV).
- **12 `src/` files reformatted** by `cyrius fmt` for the 6.5.35 formatter. The
  reformatting is continuation-line indentation only: every fmt hunk changes no
  non-whitespace and is line-count-neutral. Nine of the twelve have
  whitespace-only diffs; the other three (`acoustics_analysis`,
  `acoustics_binaural`, `dsp_spectral`) also carry the code changes above, which
  is where their line counts move. The `cyrius audit` fmt gate is clean again.

Inherited behaviour change worth recording: **`ganita_f64_tanh` gained
saturation guards.** The guard in `lib/ganita.cyr` fires at `|x| > 20`, but the
*observable* delta starts at `|x| > ~709.78` and `±inf`, where the old path
returned NaN and the new one returns ±1.0 — measured, the guarded and unguarded
paths are bit-identical for every integer in `[20, 709]`, and the first
divergence is exactly `x = 710`. Affects
`soft_clip_tanh`, `effects_distortion_process_sample` under
`DISTORTION_SOFT_CLIP`, and `physical_moog_process_sample` (where the old NaN
persisted in filter state until reset). Rust's `tanh` saturates for all inputs
— `f32::tanh` in the first two, `f64::tanh` in the moog derivative
(`rust-old/src/synth/physical.rs:382`) — so this moved naad *toward* parity.

25 new assertions pin the tanh saturation contract, negative-sample-rate
rejection, `fit_polynomial`'s guards, DCT dispatch branch coverage, and the
naad/goonj error-code namespace. One vacuous assertion was de-vacuumed (it
compared through `f64_to`, which truncates, so it held for every RT60 in
`(-1.0, 1.0)`). Suite: **38 suites / 502 assertions, 0 failed.**

**2.1.1** — abaco↔naad namespace de-collision. abaco's `dsp` module and naad
both exported bare `amplitude_to_db` / `db_to_amplitude`; in Cyrius's flat
distlib namespace they collided, blocking dhvani from bundling both libs. Renamed
naad's two onto the `naad_` prefix (matching `naad_fdn_*` / `naad_analysis_*`):
`naad_amplitude_to_db` / `naad_db_to_amplitude`. `dist/naad.cyr` × `dist/abaco.cyr`
top-level symbol intersection: **2 → 0**. Pure rename, numerics unchanged (dsp_util
/ dynamics / eq suites green at the same counts). Reviewed and rejected taking an
abaco dependency: naad owns these primitives, and abaco's DSP formulas diverge
from the `rust-old/` oracle (window `N-1` vs `N`; pole vs `1-exp` EMA coeff).

**2.1.0** — post-port audit pass. The 2.0.0 Cyrius port (all 41 modules) is
complete; 2.1.0 is the first work-loop iteration: a deep multi-agent review
(correctness / memory-safety-security / performance / refactor, adversarially
verified vs `rust-old/`) confirmed 9 findings — all repaired. Highlights: two
real bugs fixed (dsp_spectral empty-input OOB/÷0; singular `fit_polynomial` NaN
coeffs) and per-sample heap allocations eliminated on 4 hot buffer paths
(filter/reverb/oscillator/acoustics_fdn — 0 bytes/sample, verified). The
13,465-line Rust source is frozen at `rust-old/` as the parity oracle.

## Toolchain

- **Cyrius pin**: `cyrius.cyml [package].cyrius` is the source of truth — read
  it rather than trusting a number copied into prose. (This line previously
  inlined `6.3.18`, which never matched the committed manifest.)
- Build: `cyrius build src/main.cyr build/naad`
- Test **everything**: bare `cyrius test` auto-discovers and runs every
  `tests/**/*.tcyr` (measured: 40 suites). This is the only test step in
  `.github/workflows/ci.yml`.
- Test **one** suite: `cyrius test tests/<mod>.tcyr`. Both forms are real.
- **`cyrius fmt <file>.cyr` rewrites the file in place** and prints nothing.
  The non-destructive form is `cyrius fmt <file> --check`. There is no `-w`
  flag, whatever the `cyrius audit` hint text says.
- Quality commands (all verified against `cyrius help` at the current pin):
  `cyrius deps` · `cyrius build` · `cyrius test` · `cyrius fmt <f> --check` ·
  `cyrius lint <f>` · `cyrius doc --check <f>` · `cyrius vet src/main.cyr` ·
  `cyrius deny src/main.cyr` · `cyrius audit` · `cyrius bench <f.bcyr>` ·
  `cyrius fuzz` · `cyrius distlib` · `cyrius coverage`.
- **Parallel porting concurrency**: every `cyrius …` call re-resolves deps and
  races on `cyrius.lock` (verified: concurrent runs corrupt it). Serialize all
  toolchain calls behind `flock <scratch>/naad-build.lock cyrius …`.

## Dependencies

Both first-party deps are **git+tag pinned**; the exact resolved commits live
in `cyrius.lock`, which is the source of truth for what a build actually
consumed.

- **hisab** — `[deps.hisab]` in `cyrius.cyml`, git+tag, `modules =
  ["dist/hisab.cyr"]`. The symbols `src/` actually calls are `hvec3_new` /
  `HVec3_x`, `num_fft` / `num_ifft`, `num_dct` / `num_idct`, `num_rk4`,
  `calc_bspline` and `calc_catmull_rom`. (The `ganita_mat_*` least-squares path
  `dsp_spectral` leans on is *not* hisab's — `ganita` is a stdlib leaf in
  `[deps].stdlib`; `lib/hisab.cyr` defines no `ganita_mat_*`.)
- **goonj** — `[deps.goonj]` in `cyrius.cyml`, git+tag, `modules =
  ["dist/goonj.cyr"]`. Acoustics engine consumed by the `acoustics_*` wrapper
  modules. Live and wired — it is *not* commented out.
- **sakshi** — not declared directly. It arrives transitively (goonj's logging
  needs it) and is recorded in `cyrius.lock`. The vendored `lib/sakshi.cyr` is
  byte-identical to the toolchain snapshot's copy.
- stdlib leaves are declared in `[deps].stdlib` and mirrored into
  `dist/naad.deps` for consumers.

## Source

- Rust reference: 13,465 lines across 41 modules at `rust-old/` (frozen).
- Cyrius port: `src/main.cyr` (smoke) + per-module `src/*.cyr` (library,
  validated via `tests/*.tcyr`, not included by the smoke binary — same layout
  hisab/goonj use). Subdir modules (`synth/`, `oscillator/`, `acoustics/`) are
  flattened into `src/` with descriptive names.
- `[lib].modules` lists 39 entries against 40 files in `src/` — `src/main.cyr`
  is the smoke binary and is deliberately outside the bundle.

## Port progress

Per-module parity tracked in [`port-audit.md`](port-audit.md). Summary:

**41 / 41 modules ported — PORT COMPLETE** · **579 assertions green across 40
suites** · `dist/naad.cyr` bundle assembled and collision-audited to zero across
all **759** top-level symbols — `fn`, `var`, `const` and `struct` alike, against
hisab, goonj, sakshi, abaco and the whole pinned stdlib. The audit was fn-scoped
through 2.1.2, which is how the `ERR_*` `var` collision survived three releases;
`scripts/symbol-collision-check.sh` now enforces the wider scope in CI.
Re-measure rather than transcribing: `grep -cE '^(fn|var|const|struct) '
dist/naad.cyr`. Hot-path benchmarks captured; allocation budgets pinned by
`tests/allocbudget.tcyr`.

Re-measure rather than transcribing — the previous "443 fns / 463 assertions"
drifted precisely because they were hand-written:

```sh
grep -c '^fn ' dist/naad.cyr                       # top-level fns in the bundle
grep '^fn ' dist/naad.cyr | sed 's/(.*//' | sort -u | wc -l   # …and unique names
wc -l dist/naad.cyr                                # bundle size
# assertions + suites (the trailing `(` excludes the run-summary line)
cyrius test 2>&1 | grep -oP '^\d+(?= passed, \d+ failed \()' \
  | awk '{s+=$1} END {print "assertions:", s, "suites:", NR}'
```

Note the collision audit is **fn-scoped** and provably cannot see top-level
`var` collisions — see [In flight](#in-flight).

| Layer | Modules | Status |
|-------|---------|--------|
| L0 base | error, dsp_util, dsp_spectral | ✅ |
| L0 leaves | panning, mod_matrix, tuning, voice, smoothing, delay, filter, noise, granular | ✅ |
| L1 | wavetable, dynamics, reverb, eq, envelope, additive, physical, formant, fm, vocoder, drum | ✅ |
| L2 | osc_core/unison/sub/sync | ✅ |
| L3 | modulation, effects, subtractive | ✅ |
| L-acoustics | base + ambisonics, directivity, room, binaural, convolution, fdn, analysis, coupled | ✅ |

Delivered in 6 dependency-ordered parallel workflow waves (Wave 1 leaves → Wave
2 L1 → Wave 3 oscillators+dynamics+fm → Wave 4 routing → Wave 5a/5b acoustics),
plus solo bites for the foundation (error, dsp_util), the dsp_spectral block, and
the wavetable morph. Each wave integrated + independently re-verified in main.

## Tests

One `tests/<module>.tcyr` suite per ported module, ported one-for-one from that
module's Rust `#[test]` blocks (serde round-trips + Display-string tests dropped
— no serde, integer codes). Bare `cyrius test` runs them all; `cyrius test
tests/<module>.tcyr` runs one.

`tests/bundle.tcyr` is the exception — it is not a per-module port but an
integration suite that includes `dist/naad.cyr` alongside `lib/hisab.cyr` and
`lib/goonj.cyr` to pin the bundle's behaviour in the real flat namespace,
including the naad/goonj error-code overlap.

## Benchmarks

`tests/hotpath.bcyr`, via `cyrius bench` (also run by `cyrius audit`). **These
numbers are host- and boot-dependent** — the harness measures the timer floor
per run and subtracts it per sample, and that floor moves with the clocksource.
Re-run rather than comparing against a transcribed figure. Most recent local
run (hpet clocksource, floor ≈ 1.33–1.35 µs per clock read):

| Path | Per call |
|------|----------|
| `osc_next_sample` (sine) | 59–64 ns |
| `filter_svf` lowpass | 36–38 ns |
| `envelope_adsr` `next_value` | 22–23 ns |
| `noise` pink (Voss-McCartney) | 103–110 ns |

## Method

Foundation (error, dsp_util) ported solo to establish the template + prove the
toolchain. Remaining modules ported in **dependency-ordered parallel workflow
waves** (one agent per module, `flock`-serialized `cyrius test` to green),
integrated + independently re-verified in the main working tree. Modeled on
goonj's port ("six parallel workflows landed 30 of 37 modules").

## In flight

Nothing open in the port itself — 41 / 41 modules are done. The follow-ups
below are genuinely open; the scheduled view lives in
[`roadmap.md`](roadmap.md) under *Open gates*.

**Deliberately deferred by 2.1.2:**

- **`ERR_*` → `NAAD_ERR_*` prefix pass.** naad's `ERR_INVALID_FREQUENCY` (`-1`,
  `src/error.cyr:16`) shadows goonj's (`-3`) as a top-level `var` in the flat
  bundle namespace. Inert today: goonj never returns the code, and every
  consumer includes goonj *before* naad, so naad's value wins. What makes this
  worth fixing is that duplicate top-level `var`s draw **no diagnostic** from
  either `cycc` or `cyrlint` (measured) — unlike duplicate `fn`s — so the
  collision audit cannot catch the next one. Pinned by `tests/bundle.tcyr`.
- **`fit_polynomial` `nx >= 5793` cliff** inside `ganita_mat_least_squares`.
  Unchanged by this bump. Not fixed here because capping it may be a deliberate
  divergence from the oracle — that needs checking against hisab's Rust source
  first.

**Closed in 2.1.3:** the 8 undocumented public fns (`cyrius audit` now exits 0),
the `ERR_*` prefix pass, and the missing gate for the top-level `var` collision
class (`scripts/symbol-collision-check.sh`, in CI, `fn`/`var`/`const`/`struct`
scoped — verified to fail when the 2.1.2 collision is reintroduced).

**Carried forward:**

- ~~CI has no fmt / lint / deny / fuzz step~~ — **closed in 2.2.0.** CI now runs
  the changelog gate, `cyrius deps`, `cyrius build`, the symbol-collision and
  bundle-freshness gates, `cyrius audit`, `cyrius deny`, `cyrius fuzz` and
  `cyrius test`. (`cyrius fuzz` discovers `tests/*.fcyr`; no `fuzz/` directory
  is needed, which is what the earlier note got wrong.)
- **108 public fns have no caller outside their own definition** — almost all
  accessors, correctly public but **untested**. Read it as a coverage signal,
  not dead code: five of the eight fns documented in 2.1.3 were in that set, so
  the same corners were missing both docs and tests. Entire accessor families
  (`unison_*`, `subosc_*`, `wavetable_osc_*`, `wavetable_morph_*`, `physical_*`,
  `modulation_lfo_*`, `hardsync_*`, `subtractive_*`, every `*_process_buffer`)
  have zero assertions behind them.
- ~~`FILTER_*` and `VOICE_NONE` are unprefixed~~ — **closed in 2.2.0**, along
  with the six Tier-1 bare function names. Ecosystem-wide intersection is now
  zero. ⚠ The CI gate still only covers naad's own dependency bundles and the
  stdlib; sibling bundles must be passed to
  `./scripts/symbol-collision-check.sh` as arguments, so a NEW sibling collision
  would not be caught automatically.
- ~~ADR-0001's residual hole~~ — **closed in 2.2.0** by the thin-QR port; naad
  no longer references `ganita_mat_*` at all. The upstream bug is still real
  and unreported: `ganita_mat_least_squares` forms a square Q and has no failure
  return. naad is simply no longer exposed to it. **Worth filing against
  ganita.**
- ~~`tuning_note_name` domain guard~~ and ~~zero-alloc siblings for ambisonics /
  binaural / panning~~ — **both closed in 2.2.0.**
- **Coordinated consumer refresh.** 2.1.3 and 2.2.0 together rename the error
  block, the filter and voice constants, and six bare function names. Five
  vendored `lib/naad.cyr` copies (dhvani, garjan, ghurni, nidhi, prani) need a
  refresh, and dhvani/svara need their call sites updated — see the 2.2.0
  migration table.
- Broaden benchmarks (more hot paths) + capture a Rust-vs-Cyrius comparison.
- Consumer-green (dhvani/svara) once they port up the stack.
