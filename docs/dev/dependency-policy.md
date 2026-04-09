# Dependency Version Pinning Policy

## Minimum Supported Rust Version (MSRV)

The MSRV is pinned in `rust-toolchain.toml` (currently **1.82.0**).
Bumping MSRV requires a design note and a dev-log entry.

## Rules

### 1. Pin dependencies with transitive edition requirements

Dependencies whose transitive closure requires a Rust edition beyond our MSRV
must be pinned with an upper bound in `Cargo.toml`. Every such entry **must**
include an inline comment explaining the reason.

Example:

```toml
# Pinned below 4.5.24: clap_lex >=1.0 requires edition 2024
clap = ">=4.4, <4.5.24"
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

## Currently Pinned Dependencies (Phase 2.5)

| Dependency | Upper Bound | Reason |
|---|---|---|
| `clap` | `<4.5.24` | `clap_lex >=1.0` requires edition 2024 |
| `tempfile` | `<3.19` | `getrandom >=0.4` requires edition 2024 |
| `assert_cmd` | `<2.1` | Requires edition 2024 |
| `predicates` | `<3.2` | May require edition 2024 transitive deps |
| `miette` | `<7.5` | Precautionary pin |

## Updating This Document

When a pin is added, removed, or adjusted, update the table above and record
the change in the dev-log.
