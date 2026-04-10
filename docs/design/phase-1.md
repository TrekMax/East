# Phase 1 Design Document — Multi-Repo Management

**Status:** Active
**Scope:** `east init` / `east update` / `east list` / `east status` / `east manifest --resolve`

## 1. Goal

Deliver a working multi-repo management tool that can:

1. Parse a versioned `east.yml` manifest describing remotes, projects, imports, and groups.
2. Initialize a workspace (`.east/` directory) from a manifest.
3. Clone and update project repositories concurrently.
4. List projects and their status (clean, dirty, revision mismatch).
5. Resolve a manifest including transitive imports with cycle detection.

All five commands (`init`, `update`, `list`, `status`, `manifest --resolve`) must work against a fixture SDK manifest containing 3–5 git repositories.

## 2. Crates in Scope

| Crate | Role in Phase 1 |
|---|---|
| `east-manifest` | Manifest data model, YAML parsing, import resolution, cycle detection |
| `east-vcs` | Git operations via shell-out to system `git` |
| `east-workspace` | `.east/` directory layout, workspace discovery (walk upward from CWD) |
| `east-cli` | `clap` entrypoint wiring the above crates into CLI commands |

Crates **not** in Phase 1 scope: `east-config`, `east-command`, `east-runner`, `east-build`. They remain empty stubs.

## 3. Manifest Schema (v1)

```yaml
version: 1

remotes:
  - name: origin
    url-base: https://github.com/your-org

defaults:
  remote: origin
  revision: main

projects:
  - name: sdk-core
    path: sdk/core          # optional; defaults to name
    remote: origin          # optional; falls back to defaults.remote
    revision: v1.2.0        # optional; falls back to defaults.revision
    groups: [required]      # optional; defaults to []
  - name: sdk-drivers
    path: sdk/drivers
    groups: [required]
  - name: sdk-examples
    groups: [optional]

imports:
  - file: sdk/core/east.yml
    allowlist: [hal-*]      # optional glob filter on imported project names

group-filter: [+required, -optional]

commands: []   # Phase 2; ignored in Phase 1 parsing
runners: []    # Phase 3; ignored in Phase 1 parsing
```

### 3.1 Data Model

- **`Manifest`**: top-level struct. Fields: `version`, `remotes`, `defaults`, `projects`, `imports`, `group_filter`.
- **`Remote`**: `name`, `url_base`.
- **`Defaults`**: `remote` (optional), `revision` (optional).
- **`Project`**: `name`, `path` (optional, defaults to `name`), `remote` (optional), `revision` (optional), `groups` (optional).
- **`Import`**: `file` (relative path), `allowlist` (optional list of glob patterns).
- **Group filter**: list of `+group` / `-group` strings. A project is included if it belongs to at least one `+` group and no `-` group. Projects with no groups are always included.

### 3.2 Import Resolution

Imports are resolved recursively:

1. Parse the top-level manifest.
2. For each entry in `imports`, resolve the `file` path **relative to the directory containing the importing manifest**.
3. Parse the imported manifest.
4. Filter its projects through the `allowlist` (glob match on project name).
5. Merge imported projects into the resolved set (first definition wins; no override).
6. Recurse into the imported manifest's own `imports`.

**Cycle detection:** maintain a `HashSet<PathBuf>` of canonicalized absolute paths. Before parsing any manifest file, check membership and error if already visited.

### 3.3 Template Variables

Phase 1 supports only `${workspace.root}` in string values. Other namespaces (`project.*`, `config.*`, `env.*`) are deferred to later phases.

## 4. Workspace Layout

```
<workspace-root>/
├── .east/
│   ├── config.toml       # Phase 2; created empty in Phase 1
│   └── state.toml        # Tracks workspace root manifest path, last update time
├── east.yml              # The top-level manifest (user-provided or from init)
├── sdk/
│   ├── core/             # Cloned project
│   └── drivers/          # Cloned project
└── sdk-examples/         # Cloned project
```

### 4.1 Workspace Discovery

Walk upward from CWD looking for a directory that contains `.east/`. Stop at filesystem root or mount boundary. This mirrors git's `GIT_DIR` discovery logic.

## 5. Git Operations (`east-vcs`)

All git operations shell out to system `git`. No `libgit2` or `git2-rs` binding.

Required operations for Phase 1:

| Operation | Command |
|---|---|
| Clone | `git clone --single-branch -b <revision> <url> <path>` |
| Fetch | `git -C <path> fetch origin` |
| Checkout | `git -C <path> checkout <revision>` |
| Current HEAD | `git -C <path> rev-parse HEAD` |
| Current branch | `git -C <path> rev-parse --abbrev-ref HEAD` |
| Is dirty? | `git -C <path> status --porcelain` (non-empty = dirty) |
| Remote URL | `git -C <path> remote get-url origin` |

### 5.1 URL Construction

A project's full clone URL is: `<remote.url_base>/<project.name>` (or `<project.url>` if the project specifies an absolute URL — not in v1 schema but reserved).

### 5.2 Error Handling

All git commands return `Result<Output>` wrapping the process exit code, stdout, and stderr. Errors are surfaced with the full command line and stderr for diagnosis.

## 6. CLI Commands

### `east init <manifest-url-or-path> [-r <revision>]`

1. Clone the manifest repository (or copy a local manifest file) into the current directory.
   When `-r` / `--revision` is given, fetches the specified branch or tag.
2. Create `.east/` directory and `state.toml`.
3. Run `east update` implicitly.

### `east update`

1. Discover workspace root.
2. Parse and resolve the manifest (including imports).
3. Apply group filter.
4. For each included project, concurrently:
   - If not yet cloned: `git clone`.
   - If already cloned: `git fetch` + `git checkout <revision>`.
5. Display progress via `indicatif` progress bars.

Concurrency: use `tokio` tasks with a bounded semaphore (default 8 concurrent git operations).

### `east list`

1. Discover workspace root, resolve manifest.
2. Print a table of projects: name, path, revision, groups, cloned (yes/no).

### `east status`

1. Discover workspace root, resolve manifest.
2. For each cloned project, check:
   - Current HEAD vs. expected revision.
   - Working tree dirty/clean.
3. Print a table with status indicators.

### `east manifest --resolve`

1. Discover workspace root.
2. Resolve the full manifest (including all transitive imports).
3. Print the resolved manifest as YAML to stdout.

## 7. Dependencies (Phase 1)

| Crate | Dependency | Purpose |
|---|---|---|
| `east-manifest` | `serde`, `serde_yaml` | YAML parsing |
| `east-manifest` | `thiserror` | Error types |
| `east-manifest` | `glob-match` | Allowlist pattern matching |
| `east-vcs` | `tokio` (process) | Async git shell-out |
| `east-vcs` | `thiserror` | Error types |
| `east-workspace` | `thiserror` | Error types |
| `east-cli` | `clap` (derive) | CLI argument parsing |
| `east-cli` | `anyhow`, `miette` | Error diagnostics |
| `east-cli` | `tokio` (full) | Async runtime |
| `east-cli` | `tracing`, `tracing-subscriber` | Logging |
| `east-cli` | `indicatif` | Progress bars |

## 8. Testing Strategy

- **Unit tests** (in each crate's source files): data model construction, serde round-trips, group filtering, URL construction.
- **Integration tests** (in each crate's `tests/` dir): manifest parsing from YAML strings, import resolution with fixture files, git operations against temp repos created with `tempfile` + `git init`.
- **CLI integration tests** (top-level `tests/`): run the `east` binary as a subprocess against fixture manifests, verify exit codes and output.
- **Fixtures** stored under `tests/fixtures/`: minimal `east.yml` files representing a 3–5 project SDK.

## 9. Platform Considerations

- **Windows:** test CRLF handling in git output parsing; use `std::path` throughout (no hardcoded `/`); quote paths with spaces.
- **macOS:** `/var` is a symlink to `/private/var`; canonicalize paths before comparison.
- **All:** workspace discovery must handle symlinks; use `std::fs::canonicalize` for the visited set in import resolution.

## 10. Non-Goals for Phase 1

- Configuration system (`east-config`) — deferred to Phase 2.
- Extension commands (`east-command`) — deferred to Phase 2.
- Runners (`east-runner`) — deferred to Phase 3.
- CMake integration (`east-build`) — deferred to Phase 3.
- `west.yml` import converter — later phase.
- Template variables beyond `${workspace.root}`.
