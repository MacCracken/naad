# 0008 — The DSP core stays f64

**Status**: Accepted
**Date**: 2026-11-06

## Context

prani measured its Cyrius port at 15.7× slower than its f32 Rust oracle and filed this as
a P0 on 2026-08-31, because naad is the bottom of the stack: svara cannot convert without
naad, and prani cannot convert without svara. The filing named the risk itself ("the
honest prior is that codegen dominates"), and set the rule: convert one hot path first,
and if f32 buys less than about 20%, say so loudly and downgrade this rather than
propagate it up the stack.

The experiment is `filter_biquad_process_sample`, in f64 and f32, in Cyrius and Rust, on
one machine, interleaved. Tables and the harness are in
[`docs/benchmarks/f32-experiment/`](../benchmarks/f32-experiment/README.md).

| Claim | Measured |
|---|---|
| f32 is faster per sample | **No.** 18.3 ns against 18.3 ns, and 9.5 against 9.5 in a tight loop. Rust agrees: f32 is 0.5% to 3.7% slower |
| Eight lanes beat four | Partly. 31% on the bare kernel, 18% all-in. SIMD itself gives 3.3× in f64 |
| Half the memory traffic | Half the bytes, no speed: 3.6 ns against 3.7 ns past the L2 cache |
| f32 parity with the oracle | **Yes.** Bit-exact against Rust f32 |

Three things the filing did not know make it worse:

- **f32 math is slower.** ganita's tier routes through f64: `sin` is 15% slower and `exp` 93%
  slower than their f64 forms.
- **`f32` cannot be a fn return type** at cyrius 6.7.18, so an f32 crosses every call as
  raw bits in an integer register. naad's whole surface is per-sample calls.
- **Conversion has silent failure modes.** An f32 left operand reads the low 32 bits of its
  right operand, so `x * 2.0` is 0 with a warning, and naad has no type annotations today:
  943 lines of float operations, plus roughly 900 lines calling naad across six consumers.

What moves the number is shape, not width. The same arithmetic costs 22.7 ns through
accessors and one call per sample, and 9.5 ns in one loop in locals. That is 58%, three
times the threshold this question was given, and it needs no conversion.

## Decision

**The DSP core stays f64.** The P0 is downgraded by its own rule, and svara and prani
should not convert on naad's account.

## Consequences

- **Positive** — no rewrite of the library and its consumers for no measured speed.
- **Positive** — the question is closed with numbers, for the two repos it was blocking.
- **Negative** — ported tests keep tolerances where the oracle is f32. f32 parity is real,
  and this gives it up. It is a test-quality loss, not a defect.
- **Neutral** — the real gap is the compiler: 9.5 ns against Rust's 2.8 on the same loop.
- **Neutral** — reopen this if `f32` becomes a return type and ganita gets native f32
  math, and a measured workload shows the bank figures matter more than these. A win
  from eight lanes would need a structure-of-arrays bank, which f64 can use too.

## Alternatives considered

- **Convert for parity alone.** Bit-exact tests are worth having, but not at the price of
  rewriting every float in the stack, with slower math and silent operand hazards.
- **Convert only the filter banks.** The win there is 18%, on code that does not exist yet,
  and f64 banks get most of it. See the roadmap.
