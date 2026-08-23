# 0001 — `fit_polynomial` caps its sample count, diverging from the oracle

**Status**: Accepted
**Date**: 2026-08-23

## Context

`fit_polynomial(xs, ys, degree)` (`src/dsp_spectral.cyr`) ports
`rust-old/src/dsp_util.rs:265`, which delegates to `hisab::num::least_squares_poly`.
The Cyrius port delegates instead to the stdlib `ganita` leaf's
`ganita_mat_least_squares`.

Those two solvers do not allocate the same way, and that difference is reachable
from naad's public API.

**Measured, not assumed.** `rust-old/Cargo.lock` pins hisab **1.4.0**, and that
crate is on disk in the local cargo registry. Its `src/num/linalg.rs` shows
`least_squares_poly` building a column-major Vandermonde with `degree + 1`
columns and calling `qr_decompose`, which is a **thin** modified-Gram-Schmidt QR:
`Q` is `m × n` (samples × columns) and `R` is `n × n`. **No `m × m` allocation
exists anywhere in that path.** Peak Rust allocation is `O(nx · (degree+1))` —
about 139 KB at `nx = 5793, degree = 2`. Rust returns `Some(coeffs)` at
`nx = 5793`, 10 000 and 100 000 alike, and the oracle's own doc comment
enumerates its `None` cases exhaustively; a size limit is not among them.

`ganita_mat_least_squares` instead materialises a **full `nx × nx` orthogonal Q**
(`lib/ganita.cyr`), which polynomial least-squares does not require.
`ganita_mat_new` refuses any design exceeding `GANITA_MAT_MAX_ELEMS`
(33 554 430) by returning 0 — and `ganita_mat_least_squares` never checks that
return, so `ganita_mat_qr` stores through the null pointer. With
`rows == cols == nx`, `33554430 / 5793 = 5792`, so **`nx = 5793` is the exact
first failing length**. The result is a SIGSEGV out of a pure, public numeric
function.

naad may not modify `lib/` — it is vendored — and the failing allocation is
inside a dependency, invisible to the caller.

## Decision

`fit_polynomial` rejects any input where the internal Q would exceed the ganita
limit, returning the empty vec (the port's `None`), via
`if (nx > GANITA_MAT_MAX_ELEMS / nx) { return out; }`.

The bound is **derived from the upstream constant**, not hardcoded as 5793, so
it tracks a ganita bump instead of going quietly stale.

**This is a knowing divergence from the parity oracle**, and this ADR exists
because the project's standing rule is that the port matches what Rust did.
Rust succeeds on these inputs; naad now returns `None`. In scope: the
deterministic size cliff only. Out of scope: any change to results for inputs
that work today.

## Consequences

- **Positive** — a public numeric function no longer kills the process. The
  rejection reuses the same empty-vec channel every other rejected input already
  uses, so callers that already handle `None` need no change. Trading
  *divergence plus SIGSEGV* for *divergence without SIGSEGV* is strictly better
  at every input.
- **Negative** — naad returns `None` where Rust returned coefficients, for
  `nx ≥ 5793`. Any consumer fitting a polynomial over more than ~5 800 samples
  gets a different answer than the Rust library did. That is a real parity loss
  and is why this is an ADR rather than a bug fix.
- **Negative** — the cap does **not** close the whole hole. `ganita_mat_new`
  also returns 0 on plain allocation failure, so under a memory ceiling the
  268 MB Q can fail at an `nx` *below* the cap and land on the identical
  unchecked write. **Do not read this guard as making `nx = 5792` safe by
  construction.** That path stays open until ganita can report failure.
- **Neutral** — creates two follow-ups: file the missing failure return (or a
  thin-QR rewrite) upstream against ganita, and consider reimplementing a thin
  QR inside `fit_polynomial` over the Vandermonde it already builds. The latter
  would restore full parity, drop peak allocation from `nx × nx` to
  `nx × cols`, and retire this ADR — but it changes rounding for every existing
  `nx`, so it is a minor-release change, not a patch.

## Alternatives considered

- **Leave it unguarded.** Rejected: a SIGSEGV from a pure numeric function on
  ordinary-looking input is worse than any divergence. It was reachable from the
  public API with no error channel and no warning.
- **Patch `lib/ganita.cyr` to check its internal allocation.** Rejected: `lib/`
  is vendored and re-synced from the toolchain snapshot on every `cyrius deps`,
  so the fix would be silently reverted by the next bump. It belongs upstream.
- **Reimplement thin QR inside `fit_polynomial` now.** Rejected *for this
  release only*, and it is the preferred long-term fix. Modified Gram-Schmidt
  over the existing Vandermonde plus back-substitution would match hisab 1.4.0
  exactly, remove the cap, and remove the 268 MB Q. But a different algorithm
  means different floating-point rounding for **every** currently-working input,
  which is not patch-safe. Deferred to a minor release.
- **Cap silently without an ADR.** Rejected: this is precisely the case the
  "diverge only with an ADR" rule exists for. A future reader finding
  `nx > GANITA_MAT_MAX_ELEMS / nx` with no explanation would reasonably assume
  it was parity-preserving and would not know the oracle succeeds here.
