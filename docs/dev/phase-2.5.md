# Phase 2.5 Development Notes

## What Was Delivered

All four debt items resolved:

1. **Script path resolution fix** — `script:` fields now resolve relative to the declaring manifest, not the workspace root. Added `declared_in` field to `CommandDecl` (populated at resolve time) and `ManifestRelativePath` helper for consistent path resolution.

2. **miette integration** — `anyhow` removed from `east-cli`, replaced with `miette::Result` throughout. All library error types (`ManifestError`, `ConfigError`, `TemplateError`, `CommandError`, `PathResolveError`) derive `miette::Diagnostic`. Fancy terminal rendering enabled.

3. **Dependency consolidation** — All shared deps moved to `[workspace.dependencies]` with inline comments explaining version pins. Policy documented in `docs/dev/dependency-policy.md`.

4. **ManifestRelativePath** — Reusable path resolver in `east-manifest::path_resolve`, ready for Phase 3 runner config paths.

## Test Summary

| New Tests | Description |
|---|---|
| 5 | `ManifestRelativePath` resolve logic (relative, absolute, parent escape, missing, nested) |
| 2 | Script path resolution (imported manifest, regression check) |
| **Total: 130 tests** (123 from Phase 2 + 7 new) |

## What Went Well

1. **`ManifestRelativePath` was clean.** Simple struct, simple method, thorough edge case tests. Will slot directly into Phase 3 runner config.

2. **miette migration was mechanical.** `anyhow::Result` → `miette::Result`, `.context()` → `.into_diagnostic().wrap_err()`. All tests passed immediately after the swap.

3. **`declared_in` propagation via resolve().** Stamping `CommandDecl` with its declaring manifest path during BFS resolve was natural — the path is already known at that point.

## What Was Hard

1. **miette `.context()` vs anyhow `.context()`.** miette's `Context` trait requires `Diagnostic` on the error type, unlike anyhow which accepts any `Display`. Required `.into_diagnostic()` before `.wrap_err()` on non-Diagnostic errors.

2. **Duplicate crate versions.** `miette 7.4` depends on `thiserror 1.x` while we use `thiserror 2.x`. Also `unicode-width` 0.1 vs 0.2 split. Both are unavoidable transitive conflicts — documented in dev notes.

3. **Git push transient failures.** The proxy occasionally rejected pushes. Exponential backoff (2s, 4s) resolved it.

## Decisions Made

1. **`#[allow(clippy::too_many_lines)]` on `dispatch_manifest_command`.** The function grew to 103 lines after adding `ManifestRelativePath` resolution. Allowed rather than splitting, since the function is a straight-line dispatch with no shared state.

2. **miette version pinned `<7.5`.** Precautionary — future miette versions may pull in edition 2024 transitive deps. Will relax when MSRV is bumped.

## cargo tree --duplicates Output

```
thiserror v1 (from miette) vs v2 (our code) — unavoidable
unicode-width v0.1 (from miette) vs v0.2 (from indicatif) — unavoidable
```
