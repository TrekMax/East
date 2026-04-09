# Dependency Version Pinning Policy

## Minimum Supported Rust Version (MSRV)

The MSRV is pinned in `rust-toolchain.toml` (currently **1.85.0**).
Bumping MSRV requires a design note and a dev-log entry.

## Rules

### 1. Pin dependencies with transitive edition requirements

Dependencies whose transitive closure requires a Rust edition beyond our MSRV
must be pinned with an upper bound in `Cargo.toml`. Every such entry **must**
include an inline comment explaining the reason.

Example:

```toml
# Pinned below X.Y: some-dep requires edition beyond our MSRV
some-dep = ">=A.B, <X.Y"
```

### 2. Workspace-level dependency declarations

Shared dependencies are declared in `[workspace.dependencies]` in the root
`Cargo.toml`. Member crates reference them with `workspace = true`:

```toml
# In member Cargo.toml
[dependencies]
clap = { workspace = true }
```

### 3. Manual `cargo update`

`cargo update` is run manually, not on a schedule. Each update is its own
commit. If anything non-trivial changed, add a dev-log entry.

### 4. `cargo deny` enforcement

The `cargo deny` configuration lives in `deny.toml` (checked in). It enforces:

- **Duplicate-version checking** — warn level
- **License allowlist** — only approved licenses accepted
- **Git source blocklist** — no git dependencies in releases

This is initially informational. It will be promoted to a hard CI gate in
Phase 5.

## Currently Pinned Dependencies

None. All previous edition-2024-related pins were removed after upgrading
the toolchain to 1.85.0, which supports edition 2024 natively.

## Updating This Document

When a pin is added, removed, or adjusted, update the table above and record
the change in the dev-log.
