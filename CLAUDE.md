# naad — Claude Code Instructions

> **Core rule**: this file is **preferences, process, and procedures** —
> durable rules that change rarely. Volatile state (current version,
> module line counts, port progress, test counts, consumers) lives in
> [`docs/development/state.md`](docs/development/state.md).
> Do not inline state here.

## Project Identity

**naad** — Cyrius port of a Rust project (13465 lines preserved at `rust-old/`).

- **Type**: Port (Rust → Cyrius)
- **License**: GPL-3.0-only
- **Language**: Cyrius (toolchain pinned in `cyrius.cyml [package].cyrius`)
- **Version**: `VERSION` at the project root is the source of truth — do not inline the number here
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md) · [First-Party Documentation](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)

## Goal

naad (नाद — "primordial sound/vibration") **owns audio synthesis primitives for
AGNOS**: oscillators, filters, envelopes, wavetables, modulation, delay/effects,
dynamics, EQ, reverb, noise, panning, and tuning — plus feature-gated synthesis
algorithms (subtractive/FM/additive/formant/granular/physical/vocoder/drum) and
acoustics wrappers over goonj. It builds on hisab for math and serves dhvani
(sound engine) and svara (music composition).

## Current State

> Volatile state lives in [`docs/development/state.md`](docs/development/state.md) —
> port progress, surface parity, in-flight work. Refreshed every release.

This file (`CLAUDE.md`) is durable rules.

## Scaffolding

Project was scaffolded with `cyrius port`. Original Rust at `rust-old/` is the reference oracle — do not modify it; cross-check the port against it.

## Quick Start

```sh
cyrius deps                              # resolve dependencies
cyrius build src/main.cyr build/naad     # compile the smoke binary
cyrius test                              # auto-discovers and runs every tests/**/*.tcyr
cyrius test tests/<mod>.tcyr             # run ONE suite
cyrius audit                             # fmt/lint/docs/tests/bench sweep
```

Both `cyrius test` forms are real: bare is what CI runs (`.github/workflows/ci.yml`,
the only test step), the explicit path is the inner loop while working one module.

**Toolchain concurrency**: `cyrius test`/`build`/`deps` re-resolve deps and race
on `cyrius.lock` (concurrent runs corrupt it). Whenever more than one agent or shell
may touch this repo, serialize every toolchain call:
`flock <scratch>/naad-build.lock cyrius test …`.
Rust subdir modules (`rust-old/src/{synth,oscillator,acoustics}/`) flatten into
`src/` with descriptive names: `oscillator/` and `acoustics/` take a module prefix
(`osc_core.cyr`, `acoustics_fdn.cyr`), `synth/` keeps its bare names (`fm.cyr`,
`granular.cyr`). One `src/<module>.cyr` per module, with a matching
`tests/<module>.tcyr` — the four `osc_*` modules are the exception and share
`tests/oscillator.tcyr`.
Port status + conventions live in [`docs/development/port-audit.md`](docs/development/port-audit.md).

## Key Principles

- **Cross-check against `rust-old/`** — the port's correctness bar is "matches what Rust did". Diverge only with an ADR.
- **Correctness over cleverness** — if the Cyrius behavior diverges silently from Rust, the bugs win
- Test after every change, not after the feature is "done"
- **A green suite is not evidence that a toolchain or dependency bump changed nothing.**
  After any pin or dep bump, re-vendor `lib/` and verify it file-by-file against the
  *pin's own* snapshot (`~/.cyrius/versions/<pin>/lib`) — comparing old-pin against
  new-pin can show a tidy diff and still miss a half-synced tree. Inherited behavior
  changes (new guards, new failure returns from a dep) surface as segfaults and silent
  divergence, not as red tests, so re-read the dep's changelog and pin the new contract
  with assertions.
- ONE change at a time — never bundle unrelated changes
- Build with `cyrius build`, not by invoking `cycc` directly — the manifest auto-resolves deps
- Source files only need project includes — stdlib auto-resolves from `cyrius.cyml`
- `var buf[N]` = N **bytes**, not N entries

## Rules (Hard Constraints)

- **Do not commit or push** — the user handles all git operations
- **Never use `gh` CLI** — use `curl` to the GitHub API if needed
- Do not modify `rust-old/` — it's the parity oracle
- Do not skip tests before claiming changes work
- **`cyrius fmt <file>` rewrites that file IN PLACE and prints nothing** — never reach
  for it to inspect canonical output. `cyrius fmt <file> --check` is the non-destructive
  form, and the one `cyrius audit` gates on. There is no `-w` flag.
- Do not modify `lib/` files by hand — they are vendored copies (stdlib from the
  toolchain pin, deps from `[deps.*]`) that `cyrius deps` regenerates
- Do not hardcode toolchain versions in CI YAML — `cyrius = "X.Y.Z"` in `cyrius.cyml` is the source of truth

## Documentation

- [`docs/adr/`](docs/adr/) — Architecture Decision Records (*why X over Y?*)
- [`docs/architecture/`](docs/architecture/) — Non-obvious constraints
- [`docs/guides/`](docs/guides/) — Task-oriented how-tos
- [`docs/examples/`](docs/examples/) — Runnable examples
- [`docs/development/state.md`](docs/development/state.md) — Live state
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — Milestones through v1.0

