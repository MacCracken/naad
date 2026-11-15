# 0010 — Five constructors report a refused allocation

**Status**: Accepted (narrows [0006](0006-refused-allocation-stops-at-the-null-page.md))
**Date**: 2026-11-15

## Context

[ADR-0006](0006-refused-allocation-stops-at-the-null-page.md) made a refused allocation
fail-stop everywhere in naad: no checks, and the process ends at the null page. Its
argument was that nobody could act on the failure, in naad or in Rust, whose allocator
aborts.

garjan can act on it. Its 2.6.3 audit refused each allocation in turn across 9,238 cases,
one process per case, and found that its synth constructors already treat `<= 0` from a
naad constructor as a failure. The process never got that far: it died inside naad first.
garjan filed this as
[`2026-11-07-constructors-write-through-a-refused-allocation`](../development/issues/2026-11-07-constructors-write-through-a-refused-allocation.md).
It is not only a hook artifact, either. The stdlib's `alloc` returns 0 when it cannot map
a chunk, so `noise_new` faults on a real exhausted heap under `ulimit -v`.

The filing named the five functions garjan reaches: `noise_new`,
`filter_biquad_with_gain` (and through it `filter_biquad_new`), `filter_svf_new`,
`modulation_lfo_new` and `naad_vec_zeros`. A rough grep of the 2.3.3 bundle finds 98 of
107 `alloc` results unchecked. Checking all of them was offered as an alternative and not
chosen.

The same filing found a separate defect: `naad_vec_zeros(n)` never bounded `n * 8`. At
`n = 2^61 + 1` it allocated 8 bytes and zeroed 2^61 slots past them. On a real heap that
overwrites live allocations before anything faults, which is the case ADR-0006 said would
need its own check.

## Decision

**The five functions garjan reaches report a refused allocation, and nothing else changes
behaviour.**

- `noise_new`, `filter_biquad_with_gain`, `filter_svf_new` and `modulation_lfo_new` return
  `NAAD_ERR_ALLOCATION` (-6) when any of their own allocations is refused.
  `filter_biquad_new` passes on its delegate's result. Each already returned a negative
  `NAAD_ERR_*` for bad parameters, so a caller testing `naad_is_err` handles the new code
  unchanged.
- **Their callers pass the code on.** `osc_new` did not: it stored `noise_new`'s result as
  the generator, and a handle of -6 would be read far from the null page. It now returns
  the code. The other eleven call sites (vocoder, the drums, formant, EQ, de-esser,
  subtractive, the LFO-from-waveform path) already passed on any negative result.
- `naad_vec_zeros` returns 0, as `vec_new` does, on a refused allocation and for any `n`
  outside `[0, VEC_CAP_MAX]`. The bound closes the wrap for every caller. `noise_new`
  checks for the 0.
- **naad's other 28 callers of `naad_vec_zeros` stay fail-stop, and stop where they did.**
  They call `_naad_vec_zeros_or_stop`, which stores through a 0. Passing the 0 on would
  have moved the stop out of the constructor and into the first sample: measured on
  `naad_fdn_matrix_new` at a 2^31 Hz rate under a 4 GiB limit, which built an object that
  looked whole. With the helper, that case aborts in the constructor again, as on 2.3.3.
- Everywhere else, ADR-0006 stands: a refusal ends the process at the null page.
- Extending the list takes a consumer that can act on the failure, and the same proof as
  below.

## Consequences

- **Positive** — a consumer that checks constructor results can survive heap exhaustion in
  these constructors, which are the ones a synth builds per voice and per note. The wrap in
  `naad_vec_zeros` is closed, including for direct public callers.
- **Positive** — valid-input output is unchanged: 90,521 values across the touched modules
  and the hisab and goonj paths are byte-identical to 2.3.3, and 975 hostile cases give the
  same exit code and output.
- **Negative** — the line is a list, not a principle. A function's own `alloc` can be fatal
  while a refusal one call deeper returns `NAAD_ERR_ALLOCATION`. `drum_snare_new` reports a
  refused band-pass filter, for instance, but stops on its own refused `rng`.
- **Negative** — a refusal partway through keeps what was allocated before it. The bump
  allocator frees nothing, so this costs nothing it would not have cost anyway.
- **Negative** — a stop that waits for first use already existed, and still does, for raw
  allocations kept without being touched: `mod_matrix_new`'s routings vec and slots,
  `physical_moog_new`'s scratch, `wavetable_morph_new`'s two scratch buffers. They are
  unchanged since 2.3.3, and `tests/alloc_refused.tcyr` pins them, 23 requests in all.
- **Neutral** — `tests/alloc_refused.tcyr` replaces the stdlib `alloc` to refuse chosen
  requests. Each constructor's count is measured, not written down, and each loop has a
  floor so it cannot pass by refusing nothing. All 14 new checks were reverted one at a
  time, and each revert fails the suite. So do 18 of the 21 converted call sites it
  reaches. The other 3 write their vec in the same function, so they stop there either
  way.

## Alternatives considered

- **Check every allocation.** About 107 sites, new failure returns on many public
  functions, and a sweep to prove each one. Offered and not chosen. ADR-0006's arguments
  still hold for constructors nobody has asked to survive.
- **Keep ADR-0006 and fix only the wrap.** garjan needs no change either way, since it
  already treats `<= 0` as failure. It would still die inside naad, though, before its
  check runs.
- **Return the 0 from `naad_vec_zeros` to every caller.** This is the filing's suggestion
  for the 29 call sites, minus their checks. Without the checks, each caller keeps a 0 vec
  and stops at its first use. That is fail-stop in name only: the constructor reports
  success, and the crash lands in the render loop. Three constructors already have that
  shape through raw allocations (see Consequences); this would have added 28 more.
