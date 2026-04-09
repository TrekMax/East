# Phase 1 Development Notes

## What Was Delivered

All Phase 1 goals achieved:

- `east init <manifest>` — initialize workspace from local path or git URL
- `east update` — concurrent fetch/checkout with indicatif progress (8 concurrent ops)
- `east list` — tabular project listing with cloned status
- `east status` — per-project dirty/clean, HEAD SHA, branch name
- `east manifest --resolve` — print fully resolved manifest including transitive imports

## Crate Summary

| Crate | Lines | Tests | Key Design Decisions |
|---|---|---|---|
| `east-manifest` | ~350 | 41 | BFS import resolution; `glob-match` for allowlist; first-definition-wins |
| `east-vcs` | ~150 | 8 | Async shell-out via `tokio::process`; no libgit2 |
| `east-workspace` | ~80 | 7 | Walk-upward discovery mirroring git's `GIT_DIR`; canonicalized paths |
| `east-cli` | ~280 | 9 | `clap` derive; `tokio` runtime; `indicatif` progress; `Semaphore` for concurrency |

**Total: 65 tests, all passing on the development environment.**

## What Went Well

1. **TDD discipline paid off.** Writing tests first caught several serde issues early (missing `#[serde(default)]`, kebab-case renames, `skip_serializing_if`).

2. **Manifest resolution was straightforward.** BFS with a visited set was clean. Using `std::fs::canonicalize` for the visited key handled symlinks correctly.

3. **The crate boundary design worked.** `east-manifest`, `east-vcs`, and `east-workspace` have no dependencies on each other. The CLI is the only crate that wires them together.

4. **Concurrent update with `tokio` + `Semaphore` was simple.** Spawning one task per project with a bounded semaphore (8 concurrent) gave good throughput without complexity.

## What Was Hard

1. **Rust 1.82.0 + edition 2024 dependencies.** Several crate updates (tempfile 3.20+, clap 4.6+, assert_cmd 2.2+) shipped editions requiring Cargo features not available in 1.82.0. Required pinning upper bounds on dev-dependencies.

2. **Clippy pedantic + nursery.** Pedantic clippy caught legitimate issues (`#[must_use]`, `doc_markdown` backticks) but also required careful annotation. The `module_name_repetitions` lint needed `#[allow]` on error enums where the module name is `error` and the type is `ManifestError`.

3. **Git commit signing in test environments.** The test helper for creating git fixture repos needed `commit.gpgsign = false` to work in CI/development environments with signing hooks.

4. **Group filter semantics.** Deciding the exact semantics for projects with no groups (always included) vs. projects with groups that don't match any filter required careful thought. The current rule: no-group projects are always included; grouped projects need at least one `+` match and no `-` match.

## Decisions Made Mid-Phase

1. **`serde_yaml` for both parsing and output.** Used `serde_yaml` for `manifest --resolve` output instead of a separate serializer. This means the output may not preserve comments or formatting from the original, but it's semantically correct.

2. **`east init` clones manifest repo to tempdir.** When given a git URL, init clones to a tempdir, extracts `east.yml`, then discards the clone. This avoids keeping a separate "manifest repo" directory in the workspace (unlike west's T2 topology).

3. **No `state.toml` in Phase 1.** The design doc mentioned `.east/state.toml` but Phase 1 doesn't actually need it. The `.east/` directory itself is sufficient as a workspace marker. State tracking will be added when needed.

4. **BFS for import resolution instead of DFS.** BFS gives a more intuitive order (breadth-first merge of projects) and makes cycle detection trivially correct with a visited set.

## Looking Ahead to Phase 2

- **Configuration system** (`east-config`): three-layer TOML config (workspace, user, project)
- **Extension commands** (`east-command`): `commands:` section in manifest, template variable expansion
- **Template variables** beyond `${workspace.root}`: `${project.*}`, `${config.*}`, `${env.*}`
