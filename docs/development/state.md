# naad — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.1.0** — post-port audit pass. The 2.0.0 Cyrius port (all 41 modules) is
complete; 2.1.0 is the first work-loop iteration: a deep multi-agent review
(correctness / memory-safety-security / performance / refactor, adversarially
verified vs `rust-old/`) confirmed 9 findings — all repaired. Highlights: two
real bugs fixed (dsp_spectral empty-input OOB/÷0; singular `fit_polynomial` NaN
coeffs) and per-sample heap allocations eliminated on 4 hot buffer paths
(filter/reverb/oscillator/acoustics_fdn — 0 bytes/sample, verified). The
13,465-line Rust source is frozen at `rust-old/` as the parity oracle.

## Toolchain

- **Cyrius pin**: `6.3.18` (in `cyrius.cyml [package].cyrius`).
- Build: `cyrius build src/main.cyr build/naad`
- Test ONE suite: `cyrius test tests/<mod>.tcyr` (explicit path — no discovery).
- **Parallel porting concurrency**: every `cyrius …` call re-resolves deps and
  races on `cyrius.lock` (verified: concurrent runs corrupt it). Serialize all
  toolchain calls behind `flock <scratch>/naad-build.lock cyrius …`.

## Dependencies

- **hisab** (`../hisab`, `dist/hisab.cyr`) — HVec3 / HComplex / num_fft /
  calc_bspline / num_newton. Wired now (needed from L1).
- **goonj** (`../goonj`, `dist/goonj.cyr`) — acoustics engine; wired at the
  acoustics layer (transitively pulls sakshi for goonj's logging). Commented
  out in `cyrius.cyml` until then.
- Path deps during the port; pin to git+tag at the 2.0.0 release.

## Source

- Rust reference: 13,465 lines across 41 modules at `rust-old/` (frozen).
- Cyrius port: `src/main.cyr` (smoke) + per-module `src/*.cyr` (library,
  validated via `tests/*.tcyr`, not included by the smoke binary — same layout
  hisab/goonj use). Subdir modules (`synth/`, `oscillator/`, `acoustics/`) are
  flattened into `src/` with descriptive names.

## Port progress

Per-module parity tracked in [`port-audit.md`](port-audit.md). Summary:

**41 / 41 modules ported — PORT COMPLETE** · **463 parity assertions green
across 36 suites** · `dist/naad.cyr` bundle assembled (collision-audited to
zero across 443 top-level fns) · hot-path benchmarks captured.

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
— no serde, integer codes). Run a suite with `cyrius test tests/<module>.tcyr`.

## Method

Foundation (error, dsp_util) ported solo to establish the template + prove the
toolchain. Remaining modules ported in **dependency-ordered parallel workflow
waves** (one agent per module, `flock`-serialized `cyrius test` to green),
integrated + independently re-verified in the main working tree. Modeled on
goonj's port ("six parallel workflows landed 30 of 37 modules").

## In flight

Nothing — the 2.0.0 port is complete. Follow-ups for a later cycle:
- Pin `hisab`/`goonj` path deps to git+tag for a reproducible release build.
- Broaden benchmarks (more hot paths) + capture a Rust-vs-Cyrius comparison.
- Consumer-green (dhvani/svara) once they port up the stack.
