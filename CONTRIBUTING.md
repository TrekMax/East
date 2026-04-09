# Contributing to east

Thank you for your interest in contributing!

## Development Setup

1. Install Rust via [rustup](https://rustup.rs/). The pinned toolchain version will be installed automatically from `rust-toolchain.toml`.
2. Clone the repository and run `cargo check --workspace` to verify the setup.

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — new feature
- `fix:` — bug fix
- `test:` — adding or updating tests
- `docs:` — documentation changes
- `refactor:` — code restructuring without behavior change
- `chore:` — maintenance tasks (deps, CI config, etc.)
- `ci:` — CI/CD changes

Subject line must be in English. The commit body may be bilingual (English + Chinese).

## TDD Protocol

Every functional change follows **Red -> Green -> Refactor**:

1. **Red:** Write a failing test that expresses the desired behavior.
2. **Green:** Write the minimum code to make the test pass.
3. **Refactor:** Clean up while keeping tests green.

## Code Quality

- `cargo fmt --all` before committing
- `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery` must pass
- `cargo doc --no-deps -D warnings` must pass
- `#![forbid(unsafe_code)]` in every crate unless explicitly justified

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 OR MIT dual license.
