# naad — Roadmap

> Sequencing — what ships next, in what order, against what dependency gates.
> Live state lives in [`state.md`](state.md); the per-module port ledger is a
> closed record at [`port-audit.md`](port-audit.md); per-release detail is in
> the [CHANGELOG](../../CHANGELOG.md).
>
> The Rust → Cyrius port is **complete** (41/41 modules, shipped in 2.0.0). The
> milestones that delivered it are preserved in [Shipped](#shipped) at the
> bottom; this file now leads with the gates that are actually still open.

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
- [ ] **File the ganita bug upstream.** `ganita_mat_least_squares` still forms an
      `m × m` Q and still has no failure return. naad is no longer exposed to it
      after 2.2.0, but the defect is real and the next consumer will hit it.

- [ ] **Accessor test coverage.** 108 public fns have no caller outside their own
      definition — entire families (`unison_*`, `subosc_*`, `wavetable_osc_*`,
      `wavetable_morph_*`, `physical_*`, `modulation_lfo_*`, `hardsync_*`,
      `subtractive_*`, every `*_process_buffer`) have zero assertions behind
      them. This is the largest remaining gap in the project and it is a
      coverage problem, not a code problem: five of the eight fns documented in
      2.1.3 were in that set, so the same corners were missing both docs and
      tests. `cyrius coverage` reports file-level reference coverage, which does
      not see it.

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
