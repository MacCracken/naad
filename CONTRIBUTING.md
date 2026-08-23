# Contributing to naad

Thank you for your interest in contributing to naad.

**Read this first:** naad is a **Cyrius** project. There is no `Cargo.toml` at the
repository root and no cargo/clippy/rustfmt/criterion toolchain applies. The Rust
tree at `rust-old/` is a frozen parity oracle, never a build target — see
[The parity oracle](#the-parity-oracle) below.

The toolchain version is pinned in `cyrius.cyml` (`[package].cyrius`), the project
version lives in `VERSION`, and resolved dependency commits live in `cyrius.lock`.
Those three files are the source of truth; this document deliberately does not
copy their values.

## Getting Started

1. Fork the repository
2. Clone your fork
3. Install the pinned toolchain — `.github/workflows/ci.yml` shows the canonical
   install (read the pin out of `cyrius.cyml`, hand it to the upstream
   `scripts/install.sh`); never hardcode a version
4. Create a feature branch: `git checkout -b feature/your-feature`
5. Make your changes — ONE logical change per branch
6. Run the quality checks (see below)
7. Submit a pull request

```sh
cyrius deps                            # resolve [deps] into lib/, write cyrius.lock
cyrius build src/main.cyr build/naad   # compile the smoke binary
cyrius test                            # run every suite under tests/
```

`cyrius deps` re-vendors `lib/` and rewrites `cyrius.lock`. Run it when the
manifest changes; do not hand-edit either.

## Serialize every `cyrius` call

`cyrius build`, `cyrius test`, and `cyrius deps` all re-resolve dependencies and
race on `cyrius.lock` — concurrent runs corrupt it. If you are running work in
parallel (multiple terminals, a script, an agent fanning out over modules), every
toolchain invocation must go behind one shared file lock:

```sh
flock <scratch>/naad-build.lock cyrius test tests/filter.tcyr
```

This is not optional for parallel work. A corrupted `cyrius.lock` looks like a
dependency-resolution failure and wastes an afternoon.

## Quality Requirements

The one-shot sweep is:

```sh
cyrius audit          # fmt / lint / docs / tests / bench, across the project
```

`cyrius audit` must introduce no new findings before you open a pull request. Its
fmt, lint, test and bench gates are clean; the documentation gate carries a known
backlog — see [Known gate state](#known-gate-state). The individual gates
underneath it, for when you want to run just one:

| Command | What it does |
|---|---|
| `cyrius fmt <file>.cyr --check` | formatting gate — **non-destructive**, reports only |
| `cyrius lint <file>.cyr` | static analysis |
| `cyrius doc --check <file>.cyr` | documentation gate for public functions |
| `cyrius test` | auto-discovers and runs every `tests/**/*.tcyr` |
| `cyrius test tests/<mod>.tcyr` | run a single suite while iterating |
| `cyrius bench tests/naad.bcyr` | benchmark harness (also `tests/hotpath.bcyr`) |
| `cyrius vet src/main.cyr` | audit the include dependency graph |
| `cyrius deny src/main.cyr` | enforce project dependency policy |
| `cyrius coverage` | reference coverage over `src/` |
| `cyrius distlib --check` | verify `dist/naad.cyr` matches `src/` without writing |

Both `cyrius test` forms are real: bare `cyrius test` discovers and runs the whole
suite (this is exactly what CI runs), and `cyrius test tests/<mod>.tcyr` runs one
file. Use the single-suite form while iterating and the bare form before pushing.

CI (`.github/workflows/ci.yml`) runs a `CHANGELOG.md`-vs-`VERSION` check, then
`cyrius deps`, `cyrius build`, and `cyrius test`. The remaining gates are local —
run `cyrius audit` yourself; a green CI is not evidence that fmt, lint, or docs
are clean.

### `cyrius fmt` rewrites your file in place

**`cyrius fmt <file>.cyr` edits the file on disk and prints nothing.** It is not a
"print the canonical form" command, and there is no `-w` flag (if you see one
suggested in a hint string, it is wrong). The non-destructive form is:

```sh
cyrius fmt src/filter.cyr --check     # check only
cyrius fmt src/filter.cyr             # REWRITES src/filter.cyr
```

It takes one file at a time. Commit or stash before running the destructive form
so a reformat is reviewable as its own diff — formatter churn mixed into a
behavioral change makes the change unreviewable.

Same caution applies to `cyrius deps` and `cyrius distlib`: both write tracked
files (`lib/`, `cyrius.lock`, `dist/`). Run them deliberately, and keep the
resulting diff in its own commit.

`cyrius hooks install` installs the build-artifact pre-commit hook; use it to keep
`build/` out of your commits.

### Known gate state

As of 2.1.2, `cyrius audit`'s documentation gate reports 8 undocumented public
functions. That backlog is accepted; do not add to it. Every new public function
needs a doc comment, and `cyrius doc --check` on the file you touched must not
report a new one.

## The parity oracle

`rust-old/` is the frozen Rust source this project was ported from. It is the
correctness oracle.

- **Do not modify `rust-old/`.** Ever, for any reason, including "fixing" a bug in
  it. It is a reference, not code we ship.
- **The correctness bar is "matches what Rust did."** Not "is reasonable", not "is
  better numerically" — matches. Every change to a ported function must be
  cross-checked against the corresponding Rust function.
- Deliberate divergence from the oracle requires an ADR in [`docs/adr/`](docs/adr/)
  explaining why, and a test pinning the new behavior.
- Per-module parity status and per-module notes live in
  [`docs/development/port-audit.md`](docs/development/port-audit.md).

One trap worth calling out, because it has already bitten: the port widened
several Rust `u32` parameters to Cyrius `i64`. A guard transliterated verbatim as
`sample_rate == 0` is correct in Rust — where negatives are unrepresentable — and
wrong here. When the oracle's type was unsigned, re-derive the guard for the
signed type instead of copying it (`<= 0`).

## Code Standards

The porting conventions are written up in full under "Conventions established" in
[`docs/development/port-audit.md`](docs/development/port-audit.md) — read that
before touching `src/`. The rules you will hit immediately:

- **`f32` → `f64` everywhere.** hisab's `HVec3`/`HComplex` are f64-only. Test
  tolerances are loosened against the f32 oracle where bit-exactness is not
  meaningful (`NAAD_EPSILON`).
- **Float literals.** Integers go through `f64_from(n)`. Non-integers are
  module-top `var` constants holding the IEEE-754 hex bit pattern, with the
  decimal value in a trailing comment. Generate the pattern with:
  ```sh
  python3 -c "import struct;print(hex(struct.unpack('<Q',struct.pack('<d',X))[0]))"
  ```
- **`enum` → integer `var` constants** (see `src/error.cyr`, `src/dsp_util.cyr`).
- **`Result` / `Option` → error codes and sentinels.** Validators return
  `NAAD_ERR_NONE` (0) or a negative `NAAD_ERR_*` from `src/error.cyr`; value-returning
  functions use a NaN or documented sentinel. Reach for `lib/tagged.cyr` only when
  a real payload has to come back alongside the status. No panics in library
  code — return a code or a safe default.
- **`Vec<T>` / `SmallVec<T>` → stdlib `vec`** (`vec_new`/`vec_push`/`vec_len`/
  `vec_get`/`vec_set`). f64 elements sit directly in the 8-byte slots. Buffer
  parameters (`&[f32]`, `&mut [f32]`) become a `vec` handle.
- **Structs** via `#derive(accessors)` + `alloc(sizeof(T))`; Rust methods become
  free functions `TypeName_verb(self, …)`.
- **Free functions carry a `<module>_` prefix** (`filter_svf_process_sample`,
  `delay_line_read`). The distlib bundle is one flat namespace shared with hisab,
  goonj, sakshi, and the stdlib, so collision avoidance is front-loaded, not
  cleaned up later. Where a bare name is generic enough to collide across
  libraries, use the `naad_` prefix (`naad_amplitude_to_db`). Note that the
  collision audit is function-scoped and cannot see top-level `var` collisions —
  see `tests/bundle.tcyr`.
- **Modules do not `include` each other.** Each file in `src/` is self-contained;
  the build and test entry points include modules in dependency order (stdlib
  auto-resolves from `cyrius.cyml`; hisab and goonj come in as bundles).
- `var buf[N]` allocates N **bytes**, not N entries.
- Benchmarks for anything on a per-sample hot path (`tests/hotpath.bcyr`).

Adding a module means: the file in `src/`, an entry in `[lib].modules` in
`cyrius.cyml` at the right dependency position, a `tests/<mod>.tcyr` suite, and a
row in the port-audit ledger.

## Tests

- Every behavioral change needs a test that **discriminates** — one that fails if
  you revert the change. A test that passes both before and after is not evidence.
  Watch for assertions that compare through a truncating conversion; they can hold
  vacuously across the whole range you meant to pin.
- Rust `#[test]` blocks are ported one-for-one, minus serde round-trips and
  `Display`-string tests (neither survived the port).
- Test after every change, not once at the end.
- Do not lower a tolerance or delete an assertion to make a suite pass. If parity
  moved, say so in the PR and justify it against `rust-old/`.

## Commit Messages

Use clear, descriptive commit messages: what changed and why, imperative mood,
detail in the body when the subject cannot carry it. Reference issue numbers where
applicable. Keep formatter churn, dependency re-resolution, and behavioral changes
in separate commits.

## Pull Requests

Keep a PR to ONE change — never bundle unrelated work. In the description, state:

- what changed and why;
- for any change to a ported module, how it was cross-checked against `rust-old/`;
- the gates you ran (`cyrius audit`, plus any suite-specific runs) and their
  results;
- for numerics or hot-path changes, before/after benchmark numbers, with the
  measurement noted as host- and boot-dependent.

Anything user-visible also needs a `CHANGELOG.md` entry under the `## [<version>]`
heading matching `VERSION`, following the existing Keep a Changelog structure. CI
fails the branch outright if `CHANGELOG.md` has no section for the current
`VERSION` — it uses the same extractor the release workflow uses for the release
body.

## Reporting Security Issues

Do not open a public issue for a security vulnerability — follow
[`SECURITY.md`](SECURITY.md).

## Code of Conduct

Participation in this project is governed by
[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions will be licensed under
GPL-3.0-only, matching [`LICENSE`](LICENSE).
