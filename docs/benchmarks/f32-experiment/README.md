# The f32 experiment

The measurement behind [ADR-0008](../../adr/0008-the-dsp-core-stays-f64.md), run on cyrius
6.7.18 and rustc 1.99.1 on an AMD Ryzen 7 5800H, interleaved, best of 7, on a quiet
machine. Every Cyrius variant reproduces the same bits as its Rust twin of the same width.

| Scalar biquad, ns/sample | f64 | f32 |
|---|---:|---:|
| naad today: accessors, one call per sample | 22.7 | |
| Cyrius: typed fields and operators, one call per sample | 18.3 | 18.3 |
| Cyrius: one loop in locals | 9.5 | 9.5 |
| Rust: one call per sample | 6.3 | 6.5 |
| Rust: one loop | 2.8 | 2.8 |

| 64-filter bank, ns per filter per sample | f64 | f32 |
|---|---:|---:|
| Cyrius scalar | 10.5 | |
| Cyrius SSE (2 / 4 lanes) | 4.5 | 4.2 |
| Cyrius AVX2 (4 / 8 lanes) | 3.2 | 2.6 |
| Cyrius AVX2, kernel only | 2.0 | 1.4 |
| Rust, autovectorised, native | 0.41 | 0.34 |

| Other paths, ns | f64 | f32 |
|---|---:|---:|
| `sin` | 17.1 | 19.6 |
| `exp` | 11.1 | 21.4 |
| Delay line past the L2 cache, 1 M samples | 3.6 | 3.7 |

To run it, copy `biquad.cyr` and `kernels.cyr` beside a tree holding `lib/` and `dist/`, and
build them with `cyrius build`. Each takes a variant number and a sample count. `biquad.rs`
is the Rust twin, built `--release` for the scalar rows and with `-C target-cpu=native`
for the bank rows. Time each variant with its sample count, and subtract a run at 0.
