# 0006 — A refused allocation stops at the null page

**Status**: Accepted
**Date**: 2026-11-06

## Context

Two audits proposed checking `alloc` and `vec_new` at every constructor: about 23 sites
in one cluster alone, with new error returns from functions that have none today.
goonj faced the same question and answered it in its ADR-0004 by ending the process.

The risk that makes this matter is not the crash. It is a refusal that does not crash:
a silently wrong answer, or a store through 0 plus a large index that lands in the
process image instead of the null page. So the 2.3.2 sweep measured it, with a patched
`alloc` that refuses the Nth call, singly and persistently:

| Sweep | Cases | Outcome |
|---|---|---|
| Refuse the Nth allocation, 47 pipelines | 638 | 591 fault, 47 harmless (nothing needed it), **0 silent, 0 hangs** |
| Every public fn with all-zero arguments | 450 | 335 fault, 115 return, **0 hangs** |

All 926 faults were read with ptrace: every one is at the null page, offset 96 at most.

## Decision

**A refused allocation is fail-stop, and naad adds no checks for it.** Rust's allocator
aborts here too. The exception is a size that depends on input where Rust answers
something else: `fft_magnitudes` and `power_spectrum` validate first and return empty.

## Consequences

- **Positive** — no error path that nothing can test, and no new failure returns.
- **Negative** — a consumer cannot survive heap exhaustion inside naad. It could not in
  Rust either.
- **Neutral** — this rests on a measurement that a new variable-size buffer could
  invalidate. A buffer whose first store is at a large index would need its own check.
  Rerun the sweep at a P(-1).

## Alternatives considered

- **Check every allocation.** Dozens of branches on paths that cannot be reached without
  heap exhaustion, for an outcome no better than the fail-stop already measured.
- **End the process deliberately**, as goonj does. It reads better than a SIGSEGV, but
  needs a syscall, and `src/` has none outside the smoke binary (SECURITY.md).
