# 0004 — Bound delay lengths in f64 before the cast

**Status**: Accepted
**Date**: 2026-11-06

## Context

Cyrius 6.6.8 made `f64_to` saturate on x86: `NaN` gives 0, a value at or past 2^63
gives `INT64_MAX`, and one below -2^63 gives `INT64_MIN`. Before that, x86 returned
`INT64_MIN` for all three. naad moved across this in 2.2.5 (cyrius 6.6.3 to 6.6.20).

Six length paths had only a floor, `if (n < 1) { n = 1; }`, or an unchecked `+ 2`.
They worked by accident: +Inf cast to `INT64_MIN`, the floor caught it, and a
constructor quietly built a 1-sample line. With a saturating cast the same input
arrives as `INT64_MAX` and passes the floor:

| Entry point | Input | cyrius 6.6.3 | cyrius 6.6.20, unguarded |
|---|---|---|---|
| `reverb_new` | +Inf or f64::MAX pre-delay, f64::MAX sample rate | returns | aborts, `vec: capacity overflow` |
| `effects_chorus_new`, `effects_flanger_new` | +Inf delay or depth, f64::MAX sample rate | returns | aborts, `vec: capacity overflow` |
| `physical_ks_pluck` | a period past 2^63 samples | returns | 2^63 iterations, in effect a hang |
| `detect_pitch_autocorr` | f64::MAX sample rate | no pitch | aborts, `vec: index out of bounds` |
| `naad_fdn_reverb_new` | +Inf or f64::MAX room dimension | returns | takes gigabytes, then `vec: alloc failed` where they run out |

The last row also needs goonj 2.0.6, which caps these delays at
`FDN_MAX_DELAY_SAMPLES` and asks consumers to refuse such rooms first, since eight
capped lines still take gigabytes.

The test suite passed through all of this. It never drove these inputs: the ported
suites inherit the oracle's positive cases, and the negative sweep had no
floor-only length row.

The oracle is not a useful reference here. On these inputs Rust panics at
`vec![0.0; usize::MAX]`, or spins in `pluck`, and in release builds the
`min_lag + 2` overflow wraps. naad's old result, a 1-sample line for an infinite
delay, was never a decision either.

## Decision

**Every f64-to-length cast that sizes a buffer tests the f64 first, and the
constructor reports an error rather than building something it cannot honour.**

- `delay_len_overflows(samples)` in `src/delay.cyr` is the one test: 1 at or past
  `VEC_CAP_MAX`, the most `vec_push` holds before it ends the process.
- `reverb_new`, `effects_chorus_new`, `effects_flanger_new`, `physical_ks_new` and
  `physical_wg_new` return `NAAD_ERR_BUFFER_OVERFLOW`.
- `physical_ks_pluck` returns `NAAD_ERR_BUFFER_OVERFLOW` with the string silent.
  `set_frequency` can still reach this: it takes whatever `validate_frequency`
  takes, as the oracle does.
- `naad_fdn_reverb_new` returns `NAAD_ERR_COMPUTATION`, its existing out-of-range
  code, when any delay reaches `FDN_MAX_DELAY_SAMPLES`.
- `detect_pitch_autocorr` compares `min_lag >= max_lag - 2`, which cannot overflow,
  and returns its usual no-pitch value.

`NaN` still builds. It casts to 0 and floors to 1, which is what Rust's `as usize`
gives, so `delay_len_overflows` answers 0 for it deliberately.

## Consequences

- **Positive** — every row above returns instead of ending the process.
  Delays from 2^28 samples up to 2^63, which aborted on every toolchain, now report
  an error too.
- **Negative** — a caller that passed +Inf and ignored the result used to get a
  working object and now gets a negative code. Those constructors already documented
  that return, so a caller that checks is unaffected, and one that does not check
  was already exposed to bad sample rates.
- **Negative** — this diverges from the oracle's panic. The divergence is a panic
  becoming an error, which is the port's standing convention.
- **Neutral** — a line just under `VEC_CAP_MAX` is 2 GiB of samples and can still
  fail in `alloc`. This bound answers "can a vec hold it", not "is there memory".

## Alternatives considered

- **Restore the old 1-sample result** by mapping overflow to 0 at each cast. It keeps
  the pre-bump behaviour exactly, but it preserves an accident: an infinite delay
  would silently become the shortest one.
- **Cap the length**, as goonj 2.0.6 does for its delays. A cap at `VEC_CAP_MAX`
  still allocates 2 GiB per line, once per chorus voice and per reverb comb, for an
  input that has no meaningful delay.
- **Reject all non-finite parameters at the top of each constructor.** That misses
  finite overflow such as f64::MAX, and adds a gate the oracle has no equivalent of
  even for `NaN`, which Rust accepts here.
- **Leave it.** Rust dies on these inputs too, so an abort has a claim to parity. But
  naad's 2.1.3 sweep made "a numeric input never ends the process" the contract, and
  shipping a toolchain bump that quietly withdrew it is not a decision anyone made.
