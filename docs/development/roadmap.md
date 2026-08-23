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

- [~] **Clean gate enforced in CI.** `cyrius audit` (fmt · lint · docs · tests ·
      bench) passes locally, but the only *quality* step in
      `.github/workflows/ci.yml` is a bare `cyrius test`. Wire the rest in:
      `cyrius fmt <file> --check`,
      `cyrius lint <file>`, `cyrius deny src/main.cyr`, and `cyrius bench` over
      `tests/hotpath.bcyr` / `tests/naad.bcyr`. Until then the gate is available,
      not enforced — a fmt or lint regression reaches `main` unchallenged.
      (Use `--check`: bare `cyrius fmt <file>` **rewrites the file in place**.)
- [ ] **Fuzz harnesses.** `cyrius fuzz` runs `fuzz/*.fcyr`; naad has no `fuzz/`
      directory yet. Harnesses first, then a CI step. The natural first targets
      are the parsers-of-untrusted-numbers on the DSP boundary — buffer-length
      and sample-rate validation, and the spectral entry points.

### Surface polish

- [ ] **Undocumented public functions.** `cyrius audit` reports 8 on every run.
      Close them out and hold the line with `cyrius doc --check <file>`.
- [ ] **`ERR_*` prefix pass.** naad's error codes are top-level `var` constants
      sharing Cyrius's flat distlib namespace with goonj's: naad's
      `ERR_INVALID_FREQUENCY` (-1) shadows goonj's (-3) when both bundles are
      concatenated. The collision audit is fn-scoped and provably cannot see
      this — it is pinned instead by `tests/bundle.tcyr` and flagged in the
      `[lib]` note in `cyrius.cyml`. The fix is the same de-collision move
      2.1.1 made for `naad_amplitude_to_db` / `naad_db_to_amplitude`, applied
      to the error surface. Breaking for consumers, so it wants a release of
      its own with a migration note.

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
- [~] Clean gate: fmt + lint + tests + bench all green locally under
      `cyrius audit` — whose sweep *does* auto-discover the `tests/**/*.tcyr`
      suites. Still partial because CI enforces only the test step; see
      [Open gates](#release-engineering).

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
