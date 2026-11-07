# 0007 — Hostile-input gaps we accept

**Status**: Accepted
**Date**: 2026-11-06

## Context

The 2.3.2 sweep compared naad with the Rust oracle over 1,335 cases and found no
disagreement on valid input. It left a short list of hostile-input differences that are
real but not worth closing. They are written down so nobody reopens them by accident,
because the parity bar says a divergence needs an ADR.

## Decision

These stay as they are.

| Where | Difference | Why it stays |
|---|---|---|
| `wavetable_read_interpolated` | at phase past 2^63, the index wraps at i64, Rust's at u64 | the output is meaningless either way, and matching it needs u64 arithmetic on a hot path |
| `voice` age comparison | signed, where Rust's is u64 | needs 2^63 ticks: about 6.6 million years at 44.1 kHz |
| `#derive(accessors)` setters | public, and bypass what a real setter derives: `Limiter_set_ceiling_db` leaves the linear ceiling, and editing an IR through `ConvolutionReverb_ir` leaves the cached spectrum | Rust's fields are private. Use the module's own setter, or `naad_convolution_rebuild_from_ir` |
| `granular_window_value` | an unknown window id returns 1.0 | it returns a sample, so it has no error channel. `granular_set_window` refuses the id |
| `fft_magnitudes`, `power_spectrum` | past 2^27 points return empty, where Rust computes | past `ALLOC_MAX`, which is a stdlib limit |
| `naad_convolution_process_block` | samples near f64::MAX overflow inside the FFT and give NaN, where the sample path stays finite | the non-finite guard sees finite input. Real audio is nowhere near this |
| f32 range | an input that overflows or underflows f32 but not f64 gives a different answer | naad is f64 today. See the roadmap's f32 item |

## Consequences

- **Neutral** — each row is a known answer to "why does this differ", and none of them
  crashes, hangs or corrupts.
- **Negative** — the accessor row is a real sharp edge. `#derive` cannot make a field
  private to a module, and consumers do use these setters: nidhi, garjan and svara
  re-arm `Adsr` and `Lfo` that way.
- **Rule** — so a value derived from public fields is not cached unless the read
  compares it against those fields, as the unison pan gains and the subtractive cutoff
  do, or the saving is large and the exception is listed above. 2.3.2's first draft
  broke this ten times for gains of 6% or less, and an independent review caught it.

## Alternatives considered

- **Fix them all.** Each fix costs more than the case it closes, and the accessor row
  would need a language feature.
