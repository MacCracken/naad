# 0002 — Port hisab's thin QR into `fit_polynomial` rather than call ganita

**Status**: Accepted
**Date**: 2026-08-23

## Context

[ADR-0001](0001-fit-polynomial-sample-cap.md) accepted a sample cap on
`fit_polynomial` as the least-bad response to a crash. That crash had a specific
cause, established by reading the crate `rust-old/Cargo.lock` pins:

- The oracle's `hisab::num::least_squares_poly` (hisab 1.4.0,
  `src/num/linalg.rs`) solves via `qr_decompose`, a **thin** modified
  Gram-Schmidt QR. `Q` is `m × n` — samples by coefficients — and `R` is
  `n × n`. Peak allocation is `O(m · n)`. It succeeds at every `m`.
- The Cyrius port instead called the stdlib's `ganita_mat_least_squares`, which
  materialises a **full `m × m` orthogonal Q**. Polynomial least-squares does
  not need one. `ganita_mat_new` refuses designs over `GANITA_MAT_MAX_ELEMS` by
  returning 0, and `ganita_mat_least_squares` never checks that return, so
  `ganita_mat_qr` wrote through a null pointer from `m = 5793` upward.

For a degree-2 fit over 100 000 samples the two differ by roughly **2.4 MB
versus 80 TB**. The cap was never a mathematical limit; it was an artefact of a
solver doing far more work than the problem requires.

naad cannot patch `lib/` — it is vendored and re-synced from the toolchain
snapshot on every `cyrius deps`, so any fix there is reverted by the next bump.
That left three options, and 2.1.3 shipped the only patch-safe one.

## Decision

`fit_polynomial` (`src/dsp_spectral.cyr`) implements the thin QR in-tree, ported
line-for-line from hisab 1.4.0's `qr_decompose` and `least_squares_poly`:
modified Gram-Schmidt over the Vandermonde it already builds, then
back-substitution, with `R` stored `r[col][row]` exactly as the oracle stores it
so the indexing can be read against the Rust side directly.

naad no longer references `ganita_mat_*` at all. The ADR-0001 cap is removed.

Scope: the solver only. The Vandermonde is still built by incremental
multiplication rather than `powi`, retained from the original port for the
reason its comment gave — robust for negative and zero `xs`, unlike an `exp`/`ln`
pow. That is a pre-existing last-ulp difference, not something this ADR changes.

## Consequences

- **Positive** — full parity restored on the input domain. Large fits succeed as
  they do in Rust; 20 000-sample fits are asserted in `tests/hardening.tcyr`.
  ADR-0001's divergence is gone, and so is its residual hole: there is no longer
  an unchecked allocation inside a dependency that naad cannot see. Every
  allocation in the path is naad's own and is null-checked.
- **Positive** — peak allocation drops from `O(m²)` to `O(m · n)`.
- **Negative** — **different floating-point rounding for every input that
  already worked.** Modified Gram-Schmidt over the Vandermonde is not the same
  arithmetic as ganita's Householder path, so coefficients move in the last few
  ulps. This is why it is a minor release and not a patch. Nothing in naad
  asserts bit-exact coefficients, and the existing tolerance-based assertions
  pass unchanged, but a downstream consumer comparing against stored
  coefficients will see a difference.
- **Negative** — naad now owns a linear-algebra routine it previously delegated.
  If hisab fixes or improves `least_squares_poly`, naad does not inherit it.
  Mitigated by the port being line-for-line and commented as such.
- **Neutral** — the upstream bug remains worth filing: `ganita_mat_least_squares`
  still has no way to report failure, and still forms a square Q. naad is simply
  no longer exposed to it.

## Alternatives considered

- **Keep the cap (ADR-0001).** Rejected now that a parity-preserving option is
  available. The cap silently returned `None` where Rust returned coefficients,
  and it never closed the allocation-failure path at lower `m`.
- **Fix `lib/ganita.cyr`.** Rejected again, for the same reason as in ADR-0001:
  `lib/` is vendored and the fix would be reverted by the next `cyrius deps`.
- **Wait for an upstream ganita fix.** Rejected as a blocker — it is the right
  long-term outcome and should still be filed, but it gates naad's correctness
  on another project's release cycle, and naad's own oracle already specifies
  the algorithm that avoids the problem entirely.
- **Normal equations (`AᵀA x = Aᵀb`) instead of QR.** Rejected: cheaper, but it
  squares the condition number, which is a genuinely bad trade on a Vandermonde
  — the classic ill-conditioned design. It would also be a second divergence
  from the oracle's chosen algorithm, exactly what this ADR is undoing.
