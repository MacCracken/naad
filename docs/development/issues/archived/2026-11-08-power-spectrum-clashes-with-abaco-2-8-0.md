# `power_spectrum` clashes with abaco 2.8.0's, so the two cannot be built together

**Status:** ✅ **RESOLVED in 2.3.4** by option A (see *Resolution* at the end)
**Placement:** 2.3.4.
**Discovered:** 2026-11-08, moving dhvani to the newest dependency tags for its 2.3.1.
**Severity:** Medium. A hard compile error for any consumer of both. dhvani is held at abaco 2.7.0 until it is resolved.
**Affects:** naad 2.3.3 (tag `2.3.3`, `5176778`, also `main`) with abaco 2.8.0 (`1ace4d7`), built with cyrius 6.7.18.

## Summary

abaco 2.8.0 adds an FFT and, with it, `power_spectrum(re, im, out, n)`. naad has `power_spectrum(input)`. Bundles share one flat namespace, and cycc refuses a duplicate fn whose arity disagrees:

```
error:lib/naad.cyr:604:1: duplicate fn 'power_spectrum' disagrees about arity: this one takes 1, the one in lib/abaco.cyr takes 4 (last definition wins, …
```

It is the only clash between naad and abaco 2.8.0's 25 new functions. It also makes a claim in naad's `cyrius.cyml` out of date: "the bundle's whole top-level symbol set is disjoint from hisab, goonj, sakshi, abaco and the stdlib".

## Where

`src/dsp_spectral.cyr:152`, with callers in `tests/dsp_spectral.tcyr`. No other bundle in dhvani's closure calls naad's `power_spectrum`, and neither does dhvani.

## The two functions are different contracts

| | naad `power_spectrum(input)` | abaco `power_spectrum(re, im, out, n)` |
|---|---|---|
| Input | a real time-domain `vec` | split re / im arrays, already transformed |
| Does the FFT | yes | no |
| Scaling | `|X|^2 / N^2` | plain `|X|^2` |
| Output | new `vec`, bins 0..N/2 | written into `out[0..n)` |
| Failure | empty `vec` | none; returns 0 |

## Decision

abaco is the ecosystem's math library and takes priority (maintainer, 2026-11-08). abaco keeps `power_spectrum`, and naad is the side that changes.

## Options, both on naad's side

**A. Stop clashing: rename to `naad_power_spectrum`.** This is the same exception `src/dsp_spectral.cyr` already makes for `naad_chromagram`. It is small: the definition, its tests and the docs that name it. It is a public-name change for any consumer calling it directly.

**B. Convert to abaco's functionality.** Keep naad's vec-in / vec-out contract in a differently named wrapper, and build it on abaco's `rfft` plus `power_spectrum`, then apply the `1 / N^2`. Things to weigh:
- It does not remove the need to rename. A naad fn called `power_spectrum` clashes for as long as abaco has one, so A is part of B.
- naad does not depend on abaco today. It gains a direct dep, and consumers must include abaco first.
- naad's FFT comes from hisab. Moving to abaco's changes the transform, so results need re-checking against the Rust oracle. abaco's `rfft` also needs a twiddle table and returns `Ok` / `Err`.

A is required either way, and it unblocks consumers. B is the further step if naad is to take this math from abaco outright.

## Repro

In a dhvani 2.3.1 checkout, set `[deps.abaco] tag = "2.8.0"` in `cyrius.cyml`, then:

```sh
cyrius deps && cyrius build src/main.cyr build/dhvani
```

## Regression

Run naad's symbol-collision gate against abaco's newest tag. It would have caught this.

## Related, not naad's to fix

abaco 2.8.0 and shravan 2.9.3 both define `_fft_pow2(n)` with the same arity, so that pair draws only a warning, and the last definition wins silently. They differ at `n = 1`: abaco's returns 1, shravan's returns 0. With abaco included first, as in dhvani, shravan's wins and abaco's FFT loses its `n = 1` case, which is backwards under the priority above. shravan's is the one to rename, and a consumer taking abaco 2.8.0 has to deal with that as well, even after this issue is closed. Filed in shravan as `docs/development/issues/2026-11-08-fft-pow2-clashes-with-abaco-2-8-0.md`.

## Resolution

Resolved in **2.3.4** by **option A**: `power_spectrum(input)` is now
`naad_power_spectrum(input)`, with no change in behaviour. naad takes no abaco dependency,
so option B was not pursued.

- **The regression check is in CI.** `scripts/fetch-sibling-bundles.sh` fetches the newest
  x.y.z tag of all 15 siblings in dhvani's closure, abaco among them, and
  `scripts/symbol-collision-check.sh` checks the bundle against each one. Against abaco
  2.8.0 it reported `power_spectrum` before the rename, and nothing after it.
- **The gate itself was missing names.** Its pattern did not match `public` declarations
  or enums, so it saw 1,673 of hisab's 2,783 top-level names. It now reads both, enum
  members included.
- `tests/bundle.tcyr` defines abaco's 4-argument `power_spectrum` after the bundle, in
  dhvani's include order. Against the 2.3.3 bundle it fails to compile, with the arity
  error above.
- The claim in `cyrius.cyml` is rewritten to say what the gate now checks.

The `_fft_pow2` clash between abaco and shravan is not naad's, and is unchanged.
