# Phase 2.5 Design Document — Technical Debt Cleanup

**Status:** Active
**Scope:** Four targeted debt items before Phase 3. No new features, no new crates, no new subcommands.

## 1. Goal

Pay down four items of technical debt accumulated during Phase 1 and Phase 2 so that Phase 3 (build/flash/debug) starts on a clean foundation. This is a small interlude, not a feature phase.

### Debt Items

1. **Script path resolution bug fix** — `script:` field resolves incorrectly in some cases.
2. **miette integration** — structured diagnostic errors replace `anyhow` in the CLI.
3. **Dependency version pinning policy** — formalize and consolidate version pins.
4. **ManifestRelativePath helper** — extract path resolution into a reusable, testable type.

### Explicit Non-Goals

- No `state.toml` or any new persistent state file.
- No new crates added to the workspace.
- No new subcommands.
- No unrelated refactoring (if it is not listed here, it waits).
- Phase 3 features (`east build`, `east flash`, `east debug`, `east attach`, `east reset`).

### Crate Dependency Graph

The crate dependency graph is **unchanged** from Phase 2:

```
east-cli  ─────────────┬─► east-command ─► east-manifest
                       │                │
                       ├─► east-config ◄┘
                       ├─► east-workspace
                       └─► east-vcs
```

No new edges, no new crates.

## 2. Script Path Resolution Bug Fix

### 2.1 Rule

The `script:` field in a manifest-declared command resolves **relative to the manifest file that declared it**, not relative to the workspace root. This was the stated intent in Phase 2 (section 3.4), but the implementation does not propagate the declaring manifest's path through to the point of resolution.

### 2.2 Implementation

Propagate a `declared_in: PathBuf` field to `CommandDecl` at manifest load time. This path points to the manifest file (e.g., `east.yml` or an imported sub-manifest) that contains the `commands:` entry.

### 2.3 API Change

```rust
impl CommandDecl {
    /// Returns the path to the manifest file that declared this command.
    pub fn declared_in(&self) -> &Path;
}
```

This is an **additive** change. No existing public API is removed or changed in a breaking way.

### 2.4 Edge Cases

| Case | Behavior |
|---|---|
| Absolute path in `script:` | Use as-is; `declared_in` is irrelevant. |
| `../` escapes (e.g., `../shared/run.sh`) | Allowed. The joined path is canonicalized. |
| Resolved path does not exist | Error names both the manifest file and the attempted resolved path (e.g., "script `../shared/run.sh` declared in `/repo/sub/east.yml` resolved to `/repo/shared/run.sh` which does not exist"). |

## 3. miette Integration

### 3.1 What Gains miette

| Error Source | Crate | Notes |
|---|---|---|
| Manifest parse / validation errors | `east-manifest` | YAML syntax, schema violations, reserved name collisions. |
| Config TOML parse errors | `east-config` | Malformed TOML, type mismatches on read. |
| Template engine errors | `east-command` | Unknown namespace, missing key, unterminated variable. |

### 3.2 What Does NOT Gain miette (Deferred)

- `east-vcs` — git/VCS errors remain opaque wrappers for now.
- `east-workspace` — workspace I/O errors remain simple `thiserror` enums.

These will be revisited in a future phase if richer diagnostics prove valuable.

### 3.3 Library Boundary

- **Library crates** (`east-manifest`, `east-config`, `east-command`): use `thiserror` and `derive(Diagnostic)` from miette. Error types implement `miette::Diagnostic` so that structured fields (code, help, labels, source snippets) are available to any consumer.
- **CLI crate** (`east-cli`): uses `miette::Report` as the top-level error type, **replacing `anyhow` entirely**. `anyhow` is removed from `east-cli`'s `Cargo.toml`.

### 3.4 anyhow Removal

`anyhow` is removed from `east-cli`. Library crates never depended on `anyhow`, so no other `Cargo.toml` files change. The `main()` return type becomes `miette::Result<()>`.

## 4. Dependency Version Pinning Policy

### 4.1 Policy File

A new policy document is created at `docs/dev/dependency-policy.md` (bilingual English/Chinese). It codifies the rules below so that contributors have a single reference.

### 4.2 Rules

| Rule | Detail |
|---|---|
| MSRV | Pinned in `rust-toolchain.toml`. All CI jobs use this toolchain. |
| Upper bounds | Added with comments for deps known to conflict on Rust edition boundaries (e.g., a crate that requires edition 2024 while our MSRV does not support it). |
| Workspace consolidation | All version pins live in `[workspace.dependencies]` in the root `Cargo.toml`. Crate-level `Cargo.toml` files reference them with `workspace = true`. |
| Cargo update | Manual only. No automated Dependabot or Renovate. `cargo update` is run deliberately and the lockfile is committed. |
| Cargo deny | A `deny.toml` config is maintained at the workspace root. CI runs `cargo deny check` to enforce license and advisory policies. |

### 4.3 Action

Consolidate all existing version pins into `[workspace.dependencies]` with explanatory comments where a pin is non-obvious. Example:

```toml
[workspace.dependencies]
serde = "1.0"
toml = "0.8"            # pinned below 0.9: 0.9 requires edition 2024
miette = "7"
thiserror = "2"
```

## 5. ManifestRelativePath Helper

### 5.1 Location

Crate: `east-manifest`, internal module `path_resolve` (not `pub` at the crate root; accessible via `pub(crate)` or re-exported narrowly).

### 5.2 Type Definition

```rust
/// A raw path string declared in a manifest, together with the manifest
/// file it was declared in.  Resolves relative paths against the manifest's
/// parent directory.
pub struct ManifestRelativePath {
    /// Absolute path to the manifest file that contains this declaration.
    manifest_path: PathBuf,
    /// The raw string exactly as written in the manifest (e.g., "scripts/hello.sh").
    raw: String,
}
```

### 5.3 Resolution Method

```rust
impl ManifestRelativePath {
    /// Resolve the raw path to an absolute, canonicalized path.
    ///
    /// - If `raw` is absolute, use it as-is and canonicalize.
    /// - If `raw` is relative, join it onto the parent directory of
    ///   `manifest_path`, then canonicalize.
    /// - Returns `Err(PathResolveError)` if canonicalization fails
    ///   (e.g., file does not exist).
    pub fn resolve(&self) -> Result<PathBuf, PathResolveError>;
}
```

`PathResolveError` includes:

- The manifest path.
- The raw string.
- The attempted resolved path (before canonicalization failed).
- The underlying `io::Error`.

### 5.4 Phase 2.5 Usage

`ManifestRelativePath` is used **only** for `CommandDecl` `script:` resolution in this phase.

### 5.5 Phase 3 Forward-Looking

Runner config paths (e.g., OpenOCD config file paths declared in a manifest) will reuse `ManifestRelativePath` in Phase 3. **Do not implement runner path resolution in Phase 2.5.** The type is designed with that future use in mind, but the only call site in this phase is script resolution.

## 6. Testing Strategy

### 6.1 Script Path Resolution

- Use **real temporary directories** (via `tempfile` crate) with actual file creation.
- Test matrix: relative path, absolute path, `../` escape, missing file.
- Verify error messages include both the manifest path and the resolved path.

### 6.2 miette Diagnostics

- Assert on **structural `Diagnostic` fields** (error code, help text, labels), not on rendered ANSI output.
- Example: assert that a manifest parse error's `code()` returns `Some("east-manifest::parse")` and `help()` contains actionable text.
- Do **not** snapshot-test the colorized terminal output; it is fragile across miette versions and terminal widths.

### 6.3 Dependency Consolidation

- After consolidation, run `cargo tree --duplicates` and assert zero duplicates for crates we control.
- CI runs `cargo deny check` with the new `deny.toml`.

### 6.4 ManifestRelativePath

- Same real temp dir approach as script path tests.
- Unit tests in `east-manifest::path_resolve` covering: relative resolution, absolute passthrough, `../` canonicalization, missing-file error content.
