# 0003 — Convolution's block path streams instead of truncating

**Status**: Accepted
**Date**: 2026-08-28

## Context

`naad_convolution_process_block` was a faithful port of a real bug.

The FFT route computes the linear convolution of the input block with the IR —
correctly, into a buffer of length `ir_len + block_len - 1` — and then wrote only
the first `block_len` samples to the output and threw the rest away. The
discarded part is the impulse response still ringing. Nothing carried it into the
next call, so **every block boundary truncated the reverb tail**.

The oracle does exactly the same thing, at
`rust-old/src/acoustics/convolution.rs`:

```rust
for (i, o) in output.iter_mut().enumerate().take(block_len) {
    let wet = self.scratch_product[i].re as f32;
    *o = input[i] * dry + wet * self.mix;
}
```

`.take(block_len)`, no tail state, no tail field on the struct. So this is an
**inherited upstream defect, not a port defect** — which is why the port passed
its parity bar and why fixing it needs this ADR.

Proof, 4-tap all-ones IR with a single impulse followed by zeros:

| | y[0] | y[1] | y[2] | y[3] |
|---|---|---|---|---|
| `process_sample` (correct) | 1 | 1 | 1 | 1 |
| `process_block` (before) | 1 | 1−1ULP | **0** | **0** |

For any IR longer than one tap this is a discontinuity at the block rate — 86 Hz
for 512-sample blocks. A convolution reverb that restarts every block is not a
convolution reverb.

There was a second, related defect. The FFT route never touched `position` or
`input_buffer`, the ring that `process_sample` maintains, so the two entry points
kept **independent history** and could not be interleaved on one object. Nothing
documented that.

### Why the suite did not catch it

`tests/acoustics_convolution.tcyr::process_block_direct_matches_fft_path`
compares the FFT path against the direct path and passes. It processes **one
block of 8 on a fresh object** — and one block is precisely the case where the
truncating implementation is correct, because there is no earlier tail to carry.
The defect only becomes observable from the **second** block.

## Decision

Make the block path a true streaming convolution, by **overlap-save** rather than
overlap-add.

Overlap-add is the more familiar method, but it needs a saved output tail —
a new struct field, new `#derive(accessors)` symbols in a flat namespace, and a
new allocation to manage. Overlap-save needs **no new state at all**, because the
history it requires is the previous `ir_len - 1` *inputs*, and that is exactly
what the `input_buffer` ring already holds for `process_sample`.

The segment is the previous `ir_len - 1` inputs followed by this block, length
`L = ir_len + block_len - 1`, zero-padded to `fft_len = next_pow2(L)`. Circular
convolution aliases only the first `ir_len - 1` outputs — the true linear result
has length `L + ir_len - 1`, so the overhang past `fft_len` is at most
`ir_len - 2` — and those are exactly the samples discarded. The valid linear
outputs are at `[ir_len - 1, ir_len - 1 + block_len)`.

Two consequences fall out for free:

- `fft_len` is **unchanged**, so the scratch sizing, the growth policy and the
  allocation profile are all untouched.
- The block path now advances the same ring `process_sample` does, so the two
  entry points **share one history and may be interleaved** on one object.

Also fixed in passing: the FFT path sized itself on the *input* length while
writing `min(len(input), len(output))` samples, so a short output buffer silently
consumed history it never emitted. It now consumes exactly what it emits, which
is what `naad_convolution_process_block_direct` has always done.

## Consequences

- **Positive** — the block path is correct for the first time; the two entry
  points are coherent; no new public symbols, no struct change, no extra
  allocation (`tests/allocbudget.tcyr` passes unchanged); `fft_len` unchanged so
  no performance regression in the transform.
- **Negative** — a deliberate divergence from `rust-old/`, so the parity bar for
  this one function is now this ADR and its tests rather than the oracle. Anyone
  diffing the two will find them different on purpose. Rendered output of
  `naad_convolution_process_block` changes for every IR longer than one tap —
  correctly, but consumers who had unknowingly tuned around the truncation will
  hear more tail.
- **Neutral** — the same `.take(block_len)` bug exists in the Rust original and
  is now known. If that crate is ever revived, it should carry this fix too.

**Consumer impact is nil today.** ghurni, the only in-tree consumer that
evaluated this API, calls no convolution symbol at all (it models resonant bodies
with biquads; see ghurni's ADR-009). Nothing shipping is affected.

## Tests

`tests/acoustics_convolution.tcyr` gains a `process_block is continuous across
block boundaries` group. **Every assertion in it spans at least two calls**,
since that is the whole reason the original suite missed the defect:

1. The 4-tap impulse case above, block path vs per-sample path.
2. The general property — 6 blocks of 4 must equal 24 single samples, sample for
   sample, over an asymmetric IR and a non-trivial signal (with an explicit
   guard that the reference signal is not near-silent, so the comparison cannot
   pass vacuously).
3. Interleaving — two `process_sample` calls then a block of two must equal four
   `process_sample` calls.

Verified against the pre-fix implementation: 5 of the new assertions fail on it
and all 81 pass after.

## Alternatives considered

- **Overlap-add with a saved output tail.** The textbook method and the first
  one prototyped. Rejected because it needs a new struct field and therefore new
  public accessor symbols in a flat namespace, plus a lazily-grown allocation to
  manage — all to store state that overlap-save derives from a ring the struct
  already maintains. It would also have left the two entry points incoherent.
- **Document the truncation as intended single-shot behaviour.** Rejected: the
  module header describes applying an IR "to an audio stream", the type is a
  reverb, and `tests/allocbudget.tcyr` already calls it in a 64-block loop. Every
  existing use is streaming.
- **Delete the FFT path and keep only `_direct`.** It is correct, and it is the
  documented fallback. Rejected as a large capability loss for a bug that turned
  out to be a dozen lines: the direct path is O(ir_len) per sample and unusable
  for long IRs.
- **Leave it and fix it in the consumer.** Not possible — the truncation happens
  inside the transform, where a caller has no access.
