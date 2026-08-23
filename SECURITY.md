# Security Policy

## Supported Versions

The current minor line and the most recent tagged release are supported. The
pre-2.0 tags are the original Rust line — preserved at `rust-old/` as the port's
parity oracle, never built, and not supported.

Two sources of truth, deliberately not copied into this file:

- **`VERSION`** at the repository root — what "current" is.
- **`git tag --sort=v:refname`** — what has actually shipped and can therefore
  receive a patch.

This is prose rather than a table because a pinned table is present-tense policy
with no as-of stamp, so it goes stale on the next release and nobody notices.
Both failure modes are already on the shelf: this file previously listed
`0.1.x` as the only supported line — a version that appears in `CHANGELOG.md`
but was never git-tagged, so it declared the shipping line unsupported and the
only supported line unpatchable — and, as of 2026-08, the sibling
[hisab](https://github.com/MacCracken/hisab) advertised `2.9.x (current)` in
its own table while its `VERSION` read `2.11.2`.

## Attack surface

naad is a library of pure DSP primitives. Outside `src/main.cyr` — the smoke
binary, which is deliberately not in the `[lib].modules` bundle — the sources
contain **no `syscall` and no I/O of any kind**: no sockets, no filesystem
access, no FFI, no parsing of untrusted container or file formats, not even a
diagnostic write to stderr. Every entry point takes numbers and in-memory
buffers and hands numbers and buffers back.

The realistic exposure is therefore **numeric input a consumer forwarded from
somewhere less trusted** — sample rates, buffer lengths and indices, polynomial
degrees, frequencies, times, RT60s and filter coefficients — reaching functions
that index buffers or convert `f64` to an integer. Non-finite (`NaN`, `±Inf`),
negative, and out-of-range parameters are the input class that matters.

Two porting hazards account for every defect found so far, and are the first
place to look:

- **A guard transliterated from the oracle can be half a bound.** The Rust
  original types many counts as unsigned, so negatives are unrepresentable and a
  bare `== 0` test really is a full check there. The Cyrius port widened those
  parameters to signed `i64` and carried the guards across verbatim. 2.1.2
  widened five `sample_rate == 0` guards to `sample_rate <= 0`
  (`src/acoustics_fdn.cyr`, `src/acoustics_analysis.cyr`,
  `src/acoustics_binaural.cyr`). The worst case was not a crash: an FDN reverb
  constructed with a negative sample rate **returned success** and computed
  damping gains above 1, so its output grew without bound while the constructor
  reported nothing wrong. A wrong answer under a success return code is treated
  as a security-relevant defect here, not as a numerics bug.

- **A stdlib constructor's failure return being stored through.** 2.1.2 also
  fixed a null-pointer dereference in `fit_polynomial`
  (`src/dsp_spectral.cyr`): the stdlib matrix constructor gained failure returns
  where it had previously always yielded a usable header, and the null reached a
  least-squares call that dereferences it immediately. Reverting the guard makes
  `tests/dsp_spectral.tcyr` exit 139 (SIGSEGV). A `degree < 0` guard was added
  on the same path.

The one memory-safety defect predating those is 2.1.0's out-of-bounds read in
`fft_magnitudes` / `power_spectrum`: empty input fell through the FFT's
early-return into a scale loop that did a `load64` on a zero-byte allocation
with a `1/0 = +inf` scale factor.

What to expect from a report:

- **Memory safety is the top severity** — an out-of-bounds read or write, or a
  dereference of a constructor's failure return.
- **Rejecting bad input is a contract, not a nicety.** Degenerate values must
  produce an error code or a documented degenerate result — never a wild access,
  a hang, or a plausible-looking wrong answer.
- **Parity with `rust-old/` is the correctness bar.** A silent divergence from
  what the Rust original did is in scope even where both are memory-safe.
- **Out of scope**: resource use on inputs a consumer legitimately chose (a long
  render really does allocate a long render), and Cyrius's bump allocator not
  reclaiming within a run. Both are documented behaviour.

## Reporting

- Contact: **security@agnos.dev**
- Do not open a public issue, and do not post a working trigger in a public
  channel.
- Include a description, affected version, and reproduction steps.
- 48-hour acknowledgement SLA.
- 90-day coordinated disclosure.
