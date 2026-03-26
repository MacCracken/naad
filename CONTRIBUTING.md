# Contributing to naad

Thank you for your interest in contributing to naad.

## Getting Started

1. Fork the repository
2. Clone your fork
3. Create a feature branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run the quality checks (see below)
6. Submit a pull request

## Quality Requirements

All contributions must pass:

```bash
cargo fmt --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo audit
cargo deny check
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
```

## Code Standards

- `#[non_exhaustive]` on all public enums
- `#[must_use]` on all pure functions
- `#[inline]` on hot-path sample processing functions
- Zero `unwrap`/`panic` in library code — return `Result` or use safe defaults
- All public types must derive `Serialize`, `Deserialize`, `Debug`, `Clone`
- Every new type needs a serde roundtrip test
- Use `tracing` for structured logging
- Benchmarks for any performance-sensitive code

## Commit Messages

Use clear, descriptive commit messages. Reference issue numbers where applicable.

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0.
