# naad — Roadmap

> Sequencing — what ships next, in what order, against what dependency gates.
> Live state lives in [`state.md`](state.md); the per-module port ledger is a
> closed record at [`port-audit.md`](port-audit.md); per-release detail is in
> the [CHANGELOG](../../CHANGELOG.md).
>
> The Rust → Cyrius port is **complete** (41/41 modules, shipped in 2.0.0). The
> milestones that delivered it are preserved in [Shipped](#shipped) at the
> bottom; this file now leads with the gates that are actually still open.

## 🔴 P0 — move the DSP core to f32

**Filed 2026-08-31 by prani**, which measured its consequence. **naad is the
bottom of this stack, so nothing above it can move until naad does.**

### The measurement

prani's roadmap 2.0.7 ran its Cyrius port and its frozen Rust oracle on one host,
same species, same durations, same 44100 Hz. The oracle is prani 1.1.0 on
**svara 1.0.0 / naad 1.0.0 in f32**; the port is prani 2.0.6 on **svara 3.5.4 /
naad 2.2.2 in f64**:

| | Rust (f32) | Cyrius (f64) | |
|---|---:|---:|---:|
| `wolf_howl_1s` | 1.39 ms | 21.9 ms | **15.8× slower** |
| median, 13 synthesis benchmarks | | | **15.7× slower** |
| range | | | 9.9× – 17.4× |
| realtime, wolf howl | **719×** | **45.6×** | |

The band is *tight* — every vocal apparatus, every duration, 9.9×–17.4×. A
uniform ratio across unrelated code paths is the signature of something
systemic in the substrate, not a slow algorithm somewhere.

### Why this lands on naad

naad is the biquad/filter/noise layer everything else calls per sample. Its
surface is f64 on both sides, so **svara cannot convert without naad, and prani
cannot convert without svara.** Converting only the layers above naad means
widening at every per-sample call — strictly more work than doing nothing.

### The constraint that used to justify f64 is gone

The ports were written when Cyrius had no f32 math. **ganita 1.1.4 ships a
23-function f32 scalar tier** — `sin cos exp ln sqrt pow atan2 hypot cbrt floor
ceil trunc round abs neg min max clamp lerp sign log2 exp2` — and it is already
vendored in these projects' `lib/`. Check what naad actually calls against that
list before assuming a gap; prani calls **nothing** outside it.

Note `f32_add`/`f32_mul` are **not** callable functions — f32 arithmetic
dispatches through the operators on an `F32_TYID`-typed binding.

### Three reasons, and be honest about which is which

1. **SIMD width — the strongest.** cycc has `f32v8`: **eight lanes against
   `f64v4`'s four**. A vectorised f32 filter bank has twice the lanes.
2. **Half the memory traffic** per sample buffer, on a bump allocator that never
   frees.
3. **Parity.** Every consumer's Rust oracle is f32. Every tolerance loosened in
   a port's test suite exists because of this widening; f32 makes them bit-exact.

⚠ **f32 is not proven to be the cause of the 15.7×.** Three things differ in
that comparison and only one is float width — the other two are three major
versions of the DSP stack and LLVM `--release` against cycc. **The honest prior
is that codegen dominates and float width is second.** Do not open this expecting
a 15× win. Measure a single hot filter path in f32 before converting the module.

### Suggested first step

Convert **one** hot per-sample path — `filter_biquad_process_sample` is the
obvious candidate — behind whatever the smallest honest experiment is, and
publish the delta. If f32 buys less than ~20% there, say so loudly and this P0
gets downgraded rather than propagated up the stack. **A measured "no" is a good
outcome and closes the question for svara and prani too.**

**Blocked on**: nothing. naad is the bottom.
**Blocks**: svara's f32 conversion, and prani 2.1.0 Lane A.

---

## Open gates

Not yet scheduled against a version. These are the follow-ups the project has
committed to — nothing here is speculative, and no milestone is invented.

### Release engineering

- [x] **Clean gate enforced in CI** — shipped in 2.2.0. CI runs the changelog
      gate, the symbol-collision gate, the bundle-freshness gate, `cyrius audit`
      (fmt · lint · docs · tests · bench), `cyrius deny` and `cyrius fuzz`
      before `cyrius test`. Enforcing `audit` only became possible once it
      started exiting 0 in 2.1.3.
      (When running fmt by hand use `--check`: bare `cyrius fmt <file>`
      **rewrites the file in place**.)
- [x] **Fuzz harness** — shipped in 2.2.0. `cyrius fuzz` discovers
      `tests/*.fcyr` (no `fuzz/` directory is required — the earlier note here
      was wrong). `tests/naad.fcyr` drives the DSP boundary with the values that
      actually break Cyrius ports and is verified to fail when any of the 2.1.3
      id guards is reverted. 1273 checks, wired into CI.

### Surface polish

- [x] **Undocumented public functions** — closed in 2.1.3. `cyrius audit` now
      exits **0** (fmt · lint · docs · tests · bench all clean), the first time
      in the project's history. Seven of the eight sat *second* under a shared
      banner that already named them, so none was genuinely undocumented — the
      docs gate attributes a banner to the first `fn` that follows it. Hold the
      line with `cyrius doc --check <file>`.
- [x] **`ERR_*` prefix pass** — shipped in 2.1.3. naad's six error constants
      were top-level `var`s sharing Cyrius's flat distlib namespace with
      goonj's, and naad's `ERR_INVALID_FREQUENCY` (-1) shadowed goonj's (-3)
      when both bundles were concatenated. The collision audit was fn-scoped
      and provably could not see it. Renamed to `NAAD_ERR_*`, the same
      de-collision move 2.1.1 made for `naad_amplitude_to_db` /
      `naad_db_to_amplitude`. The bundle's top-level symbol set is now disjoint
      from hisab, goonj, sakshi, abaco and the stdlib in all five directions.
      Breaking for consumers — see the 2.1.3 migration note.

- [x] **Prefix the remaining bare public names** — shipped in 2.2.0. 2.1.1 took
      the dB helpers, 2.1.3 the error block, 2.2.0 the `FILTER_*` and `VOICE_*`
      constants and the six Tier-1 function names. Ecosystem-wide top-level
      symbol intersection is **zero** across all 126 sibling bundles and the
      pinned stdlib. The compound-prefixed families (`WAVEFORM_*`, `NOISE_*`,
      `MODULATION_LFO_*`, …) were deliberately left alone — measured at zero
      risk, and prefixing them buys verbosity, not safety.
- [x] **Retire ADR-0001** — shipped in 2.2.0. `fit_polynomial` ports hisab
      1.4.0's thin QR in-tree; no square Q is formed, the cap is gone, and large
      inputs succeed as they do in Rust. Superseded by ADR-0002.
- [x] **File the ganita bug upstream** — filed 2026-08-23 in the **ganita** repo
      (`docs/development/issues/2026-08-23-least-squares-unchecked-q-alloc.md`,
      with a verified SIGSEGV repro alongside it). `ganita_mat_least_squares`
      forms an `m × m` Q it does not need and never checks its own `mat_new`, so
      1.1.4's CWE-190 guard turned an oversized design into a null write. naad
      stopped being exposed at 2.2.0; the report stands on its own merits.
      Awaiting an upstream fix — re-check before any future ganita re-pin.

- [x] **Accessor test coverage** — shipped in 2.2.1. 182 untested public
      functions to **zero**; all 440 are now referenced by a test, suite 579 →
      2340 assertions. Expectations were re-derived from `rust-old/` rather than
      transcribed from naad's output (transcribing would freeze an existing bug
      in as correct), and an independent audit swept every added assertion for
      vacuity: 0 `f64_to` comparisons, 0 tautologies, 155/155 float constants
      verified, one genuine vacuity found and fixed. 12 `_`-prefixed internal
      helpers remain unreferenced by design.

### Allocation-free hot paths

- [ ] **An allocation-free four-output SVF core.** **Filed 2026-08-31 by nidhi**,
      which measured it.

      `filter_svf_process_sample` (`src/filter.cyr:386`) ends with
      `alloc(sizeof(SvfOutput))` — **32 bytes per call**, measured over 44,100
      calls (1,411,200 B). `filter_svf_process_sample_lowpass` (`:420`) measures
      **0**. So the escape hatch exists for low-pass and only for low-pass;
      high-pass, band-pass and notch have none.

      For a sampler that is not a throughput cost, it is unbounded growth:
      `lib/alloc.cyr`'s free is a no-op, so a voice filtering at 44.1 kHz never
      gives the memory back. nidhi at 64 voices x 2 channels reaches
      **180.6 MB/s** (64 x 2 x 44100 x 32 = 180,633,600). It removed every other
      per-sample allocation from its render path in 2.0.2 and asserts a
      zero-byte delta across a rendered block; this is the one source left that
      is not on its side of the line.

      Measured end to end on identical 8-voice 512-frame block renders differing
      only in filter type: **1.528 ms** low-pass vs **1.819 ms** high-pass, a
      **19 %** penalty.

      **Requested shape** — `_filter_svf_compute_into(self, input, out4)`,
      writing low/high/band/notch into caller-owned scratch, so a voice can hoist
      one slot for its lifetime. naad already uses this pattern in at least six
      places: `reverb_process_core`, `panning_pan_mono_into` — whose header
      explicitly names `reverb_process_core` as the split being followed —
      `naad_ambisonics_encode_sample_into`, `naad_binaural_process_sample_into`,
      `_bspline_eval_1d_into`, `_naad_fdn_hadamard8_into`. The request is to
      complete a pattern, not introduce one.

      **Two workarounds exist and neither is a fork**, so this is not blocking
      nidhi — it is asking naad to make the obvious route the fast one:
      (a) route those modes through `filter_biquad_process_sample` (`:2673` in
      the 2.2.2 bundle), which is allocation-free and covers HIGHPASS/BANDPASS/
      NOTCH, at the cost of a different topology and therefore an ADR;
      (b) call `_filter_svf_compute_lowpass` and recover the other three outputs
      from `k` and the pre/post `ic1eq` via the derive accessors — bit-identical,
      but it depends on naad's integrator update staying as it is.

      **Acceptance:** numerics bit-identical to the current
      `filter_svf_process_sample` fields. nidhi verifies render output
      byte-for-byte against a 24,576-sample differential, so any drift surfaces
      immediately.

      *Withdrawn from this filing:* nidhi also reported that `Adsr` and `Lfo`
      could not be re-armed and asked for setters. **That was wrong.** Both
      structs are `#derive(accessors)`, so every parameter is already settable,
      and `modulation_lfo_set_frequency` already exists as a validating setter.
      nidhi took per-note allocation from 264 B to **0** against naad 2.2.2 with
      no upstream change. No action needed here.

### Downstream

- [ ] **Consumer-green (dhvani / svara).** Both port up the stack after naad.
      naad is not finished as a dependency until they build and test against
      it — that is the exercise that finds the surface gaps naad's own suites
      cannot.
- [ ] **Retire `rust-old/`.** The 13,465-line oracle stays frozen in-tree until
      consumers are green and no open parity question still needs it. Deleting
      it is the last step of the port, not the first.

## Out of scope

- New DSP features beyond the Rust surface — parity first, enhancements later.
- SIMD/`f64v4` hot-path vectorization (host `dhvani` owns SIMD dispatch).
- Migrating downstream consumers *for* them — naad's obligation is to be
  buildable and green against their port, not to do it.

## Shipped

### 2.0.0 criteria (port complete) — ✅ SHIPPED

- [x] All 41 Rust modules ported function-for-function (parity vs `rust-old/`)
- [x] Every module has a `tests/<mod>.tcyr` suite, all green (each Rust
      `#[test]` ported one-for-one, minus serde/Display) — **472 assertions /
      37 suites** *as of the 2.0.0 tag*, counting `tests/bundle.tcyr` alongside
      the 36 per-module parity suites; the 2.0.0 CHANGELOG entry and
      [`port-audit.md`](port-audit.md) record the parity-suites-only figure,
      **463 / 36**. (The suite has grown every release since; `state.md`
      carries the current count.)
- [x] dsp_util hisab-interop block landed (→ `dsp_spectral.cyr`)
- [x] `[deps.goonj]` wired; acoustics layer green (8 wrappers + base)
- [x] `[lib]` distlib bundle `dist/naad.cyr` assembled; collision audit clean
      (0 dups / 443 fns *as of the 2.0.0 tag*); bundle smoke test green
- [x] Benchmarks captured for hot paths (oscillator, filter, envelope, noise)
- [x] CHANGELOG complete; `VERSION` bumped to **2.0.0**
- [x] deps pinned git+tag — hisab and goonj are both git+tag pinned in
      `cyrius.cyml`, with the resolved commits recorded in `cyrius.lock`
- [x] Clean gate: fmt + lint + tests + bench all green under `cyrius audit` —
      whose sweep *does* auto-discover the `tests/**/*.tcyr` suites. Was partial
      at the 2.0.0 tag (CI enforced only the test step) and stayed partial until
      the docs gate closed in 2.1.3; CI has enforced the full sweep plus `deny`
      and `fuzz` since 2.2.0.

### M0 — Port scaffold — ✅ shipped 2026-06-30

- `cyrius port` scaffold; Rust source frozen at `rust-old/`.
- `cyrius.cyml` deps wired (stdlib + hisab); toolchain pinned **6.3.18** — the
  pin *at that date*; `cyrius.cyml [package].cyrius` is the current one.
- Tracking docs: state.md, port-audit.md, roadmap.md.

### M1 — Foundation — ✅ shipped

- `error.cyr` (codes + flush_denormal + tolerances) — 23 assertions.
- `dsp_util.cyr` scalar core (dB/interp/windows/PRNG/LUT/poly) — 36 assertions.
- 59 assertions green at the close of M1. Proved the toolchain and the
  `flock`-serialized parallel-porting method.

### M2 — L0 leaves + L1 (Waves 1–3) — ✅ shipped

- **Wave 1**: panning, mod_matrix, tuning, voice, smoothing, delay, filter,
  noise, granular.
- **Wave 2**: reverb, eq, dynamics, wavetable, envelope, additive, physical,
  formant, fm, vocoder, drum (some needing hisab HVec3).
- Gate honoured throughout: each module green before its dependents started.

### M3 — Oscillators & routing (L2–L3) — ✅ shipped

- oscillator core/unison/sub/sync (core↔unison mutually referential — ported
  together), then modulation, effects, subtractive.

### M4 — Acoustics (goonj wrappers) — ✅ shipped

- `[deps.goonj]` wired; acoustics mod-helper + the 8 acoustics wrappers ported.

### M5 — dsp_util spectral block + bundle + release — ✅ shipped

- dsp_util's hisab-interop functions finished (→ `dsp_spectral.cyr`);
  `dist/naad.cyr` assembled; collision audit; benchmarks; **bump to 2.0.0**.

### Post-2.0.0

Work-loop releases past the port are recorded in the
[CHANGELOG](../../CHANGELOG.md) and summarised in [`state.md`](state.md), not
tracked as milestones here.
