# Phase 2.7 Development Notes

## What Was Delivered

Quality and infrastructure improvements across four areas:

### 1. Doc-tests for Core Crates

Added runnable doc examples to all major public APIs that previously had none:

| Crate | Types/Methods | Count |
|---|---|---|
| `east-config` | `ConfigValue`, `ConfigStore`, `ConfigStore::from_toml_str` | 3 |
| `east-manifest` | `Remote`, `Project`, `Manifest::from_yaml_str` | 3 |
| `east-workspace` | `Workspace` (init + discover) | 1 |
| `east-vcs` | `Git` (compile-only `no_run`) | 1 |
| `east-command` | `CommandRegistry::from_manifest` | 1 |

Doc-tests went from **1** (only `Config`) to **9** across the workspace.

### 2. Unified Error Handling in `east-vcs`

`east-vcs` was the last crate using bare `thiserror` without `miette::Diagnostic`. Now:

- `VcsError` derives `miette::Diagnostic`
- Each variant has `#[diagnostic(help(...))]` with actionable hints:
  - `GitFailed` → "check that the repository exists and the revision is valid"
  - `Io` → "ensure git is installed and available on PATH"
- `miette` added to `east-vcs/Cargo.toml` dependencies

This completes the miette migration started in Phase 2.5 — all library crates now produce rich diagnostics.

### 3. CI Coverage Reporting

Added a `coverage` job to `.github/workflows/ci.yml`:

- Uses `cargo-llvm-cov` (via `taiki-e/install-action`) for instrumented coverage
- Generates LCOV output
- Uploads to Codecov via `codecov/codecov-action@v5`
- `fail_ci_if_error: false` — coverage upload failures don't block CI

### 4. Configurable Concurrency for `east update`

The hardcoded `Semaphore(8)` in `do_update()` is now configurable:

- Reads `update.jobs` from the layered config system
- Falls back to `DEFAULT_CONCURRENT_GIT` (8) if not set
- Minimum value clamped to 1
- Users set it via: `east config set --int update.jobs 16`

## Test Summary

- 9 new doc-tests (previously 1)
- All existing ~155 tests continue to pass
- Clippy: zero warnings
- **Total: ~164 tests**

## What Went Well

1. **Doc-tests doubled as API validation.** Writing examples exposed that the public API surface is clean and ergonomic — all examples are short and self-contained.

2. **miette on VcsError was trivial.** Just adding the derive and help attributes. No error-path refactoring needed.

3. **Config-driven concurrency required minimal code.** The layered config system already supports integer values and dotted keys. Reading `update.jobs` was a 5-line change.

## Decisions Made

1. **`no_run` for `east-vcs` doc-test.** Git operations require a real remote, so the example is compile-checked only. Unit tests already cover runtime behavior with temp repos.

2. **`cargo-llvm-cov` over `cargo-tarpaulin`.** llvm-cov is faster, supports more targets, and produces cleaner LCOV output. It requires the `llvm-tools-preview` rustup component.

3. **`fail_ci_if_error: false` for Codecov.** Coverage upload is best-effort — Codecov outages should not block merges.

4. **Minimum jobs clamped to 1.** Setting `update.jobs` to 0 or negative values falls back to 1 rather than panicking on `Semaphore::new(0)`.
