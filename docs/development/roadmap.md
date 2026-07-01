# naad — Roadmap

> Milestone plan for the Rust → Cyrius port (→ 2.0.0). State lives in
> [`state.md`](state.md); per-module parity in [`port-audit.md`](port-audit.md);
> this file is the sequencing — what ships, in what order, against what
> dependency gates.

## 2.0.0 criteria (port complete) — ✅ SHIPPED

- [x] All 41 Rust modules ported function-for-function (parity vs `rust-old/`)
- [x] Every module has a `tests/<mod>.tcyr` suite, all green (each Rust
      `#[test]` ported one-for-one, minus serde/Display) — **472 assertions / 37 suites**
- [x] dsp_util hisab-interop block landed (→ `dsp_spectral.cyr`)
- [x] `[deps.goonj]` wired; acoustics layer green (9 wrappers + base)
- [x] `[lib]` distlib bundle `dist/naad.cyr` assembled; collision audit clean
      (0 dups / 443 fns); bundle smoke test green
- [x] Benchmarks captured for hot paths (oscillator, filter, envelope, noise)
- [x] CHANGELOG complete; `VERSION` bumped to **2.0.0**
- [ ] deps pinned git+tag (path deps during dev — follow-up before tag push)
- [~] Clean gate: fmt + lint + tests + bench all green (`cyrius audit`
      auto-discovery N/A — naad uses explicit-path suites)

## Milestones

### M0 — Port scaffold — ✅ shipped 2026-06-30

- `cyrius port` scaffold; Rust source frozen at `rust-old/`.
- `cyrius.cyml` deps wired (stdlib + hisab); toolchain pinned 6.3.18.
- Tracking docs: state.md, port-audit.md, roadmap.md.

### M1 — Foundation — ✅ (error + dsp_util core, 59 assertions green)

- `error.cyr` (codes + flush_denormal + tolerances) — 23 assertions.
- `dsp_util.cyr` scalar core (dB/interp/windows/PRNG/LUT/poly) — 36 assertions.
- Proved toolchain + the `flock`-serialized parallel-porting method.

### M2 — L0 leaves + L1 (Waves 1–3)

- **Wave 1** (in flight): panning, mod_matrix, tuning, voice, smoothing, delay,
  filter, noise, granular.
- **Wave 2**: reverb, eq, dynamics, wavetable, envelope, additive, physical,
  formant, fm, vocoder, drum (some need hisab HVec3).
- Gate: each module green before its dependents start.

### M3 — Oscillators & routing (L2–L3)

- oscillator core/unison/sub/sync (core↔unison mutual — port together),
  then modulation, effects, subtractive.

### M4 — Acoustics (goonj wrappers)

- Wire `[deps.goonj]`; port acoustics mod-helper + the 8 acoustics wrappers.

### M5 — dsp_util spectral block + bundle + release

- Finish dsp_util's hisab-interop functions; assemble `dist/naad.cyr`;
  collision audit; benchmarks; **bump to 2.0.0**.

## Out of scope (for 2.0.0)

- New DSP features beyond the Rust surface — parity first, enhancements later.
- SIMD/`f64v4` hot-path vectorization (host `dhvani` owns SIMD dispatch).
- Downstream consumer (dhvani/svara) migration — they port up the stack after.
