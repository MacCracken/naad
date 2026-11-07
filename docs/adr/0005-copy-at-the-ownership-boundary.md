# 0005 — Copy at the ownership boundary

**Status**: Accepted
**Date**: 2026-11-06

## Context

Rust moves or copies what a constructor keeps. `Wavetable::from_samples(samples:
Vec<f32>)` takes the vec, `add_routing(routing: ModRouting)` copies a `Copy` struct,
and either way the caller can no longer reach the stored value. The port passed the
pointer and kept it. Nothing in the language objects, and no test noticed, because a
ported suite never reuses a vec after handing it over.

The 2.3.2 audit reproduced what that costs on ordinary use:

| Entry point | Observed |
|---|---|
| `wavetable_from_samples` | reusing the vec rewrites the table; truncating it divides by zero (SIGFPE) |
| `wavetable_morph_new` | pushing to the vec overruns scratch sized at construction: memory corruption |
| `mod_matrix_add_routing` | adding a routing twice aliases the slots; one edit reaches both |
| `tuning_table_custom` | later writes reach the table and bypass its validation |
| `naad_convolution_from_ir`, `rebuild_from_ir` | growing the vec outruns the ring and aborts |
| `envelope_catmull_rom_new`, `envelope_multistage_new` | truncating the points aborts mid-render; writes bypass the monotonic-time check |
| `granular_set_source`, `formant_synth_apply_params` | same shape |

## Decision

**Anything kept past the call is copied at the boundary, where Rust takes it by value.**
`naad_vec_copy` is the one way to do it. Deep where Rust's elements move too.

`Wavetable` goes one step further and is immutable: `wavetable_from_samples` copies in,
`wavetable_samples` and `wavetable_morph_tables` copy out, and oscillators may then
share a table by pointer, as they could never observe each other.

## Consequences

- **Positive** — a caller's vec is its own again, and validation cannot be bypassed
  after the fact. Six crash paths and one memory corruption close at once.
- **Negative** — one more copy at construction. On a bump allocator that memory is
  never returned, so `granular_set_source` with minutes of audio holds it twice. Rust
  moves instead, which Cyrius cannot express.
- **Negative** — `wavetable_samples` and `wavetable_morph_tables` now allocate per call.
  Read a table once, not per sample.
- **Negative** — `mod_matrix_routing_mut` is per-index: pointer identity between what
  was added and what is stored is gone, and tests that pinned it changed.

## Alternatives considered

- **Document "the callee owns it now".** It is what Rust's types enforce and what this
  codebase never enforced. A documented rule with no check is how these bugs were made.
- **Copy only where a crash was reproduced.** The class is mechanical, so the next
  instance would be the same bug found later.
