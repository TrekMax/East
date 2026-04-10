# Phase 2.6 Design Document — Topology Correction

**Status:** Active
**Scope:** Fix the workspace topology so the manifest lives in a real git repository, not as a loose file at workspace root. Breaking change for existing workspaces.

## 1. Why This Phase Exists

Phase 1 made the choice to extract `east.yml` from the manifest repository and discard the clone. This was a modeling error with these costs:

- No manifest history (loose file, no git context).
- No way to update the manifest from upstream.
- No PR workflow for manifest changes.
- Silent divergence risk between local and upstream manifest.
- **Files that naturally live alongside `east.yml`** (OpenOCD configs, toolchain files, build scripts, application source in T3 topology) are lost.

Phase 2.6 adopts a model inspired by west's T1/T2/T3 topologies: the manifest repository is always a real git repo inside the workspace, sibling to `.east/`.

The author's primary scenario is **T3 (Application)**: the manifest repo IS the application, `east.yml` declares dependencies, and `east build` builds the application.

## 2. Workspace Layout

```
<workspace-root>/
├── .east/
│   ├── config.toml      # includes [manifest] section
│   └── state.toml
├── <manifest-repo>/     # real git repo, sibling of .east/
│   ├── .git/
│   ├── east.yml
│   └── (OpenOCD cfg, src/, CMakeLists.txt, etc.)
├── <project-a>/         # fetched by east update
├── <project-b>/
└── ...
```

Key properties:

- `.east/` marks the workspace root (discovery unchanged from Phase 1).
- Manifest repo is a **real git repo**, never a bare directory.
- Manifest repo is a **sibling** of `.east/`, not a child.

## 3. `east init` — Three Modes

### Mode L — Local existing repo

```
east init -l [--mf FILE] <local-path>
```

- `<local-path>` must exist and contain the manifest file (default `east.yml`).
- `.east/` created in **parent** of `<local-path>`.
- Does NOT auto-run `east update`.

### Mode M — Clone from remote

```
east init -m <url> [--mr REV] [--mf FILE] [<workspace-dir>]
```

- Clones `<url>` to `<workspace-dir>/<repo-name>/`.
- `<repo-name>` derived from URL basename minus `.git`, or from `self.path` if present in cloned manifest.
- If `--mr` given, checks out that revision.
- `.east/` created in `<workspace-dir>`.
- Does NOT auto-run `east update`.

### Mode T — Template (default)

```
east init [<dir>]
```

- `<dir>` defaults to `manifest`.
- Creates template `east.yml`, `.gitignore`, runs `git init`.
- Does NOT add remote or make initial commit.
- `.east/` created in CWD.

In all modes: `.east/` already exists = hard error unless `--force`.

## 4. `config.toml` — `[manifest]` Section

```toml
[manifest]
path = "my-app"        # workspace-relative path to manifest repo
file = "east.yml"      # manifest filename, relative to manifest.path
```

- Written by `east init`, read by `Workspace::load()`.
- Workspace config layer only.
- Validation: `path` must be relative, non-empty, no `..`, no absolute. Forward slashes only in TOML.

## 5. Manifest `self:` Section (optional)

```yaml
version: 1
self:
  path: my-app          # hint about expected workspace path
```

- Entirely optional. Manifests without it work unchanged.
- Mode L: mismatch with init arg basename = warning (not error).
- Mode M: if present, overrides URL-derived repo-name for the clone directory.
- Mode T: included as commented-out documentation in template.
- Future reserved fields: `description`, `maintainers`, `repo-url` — parsed and ignored.

## 6. Workspace API Changes

New methods on `Workspace`:

```rust
pub fn manifest_repo_path(&self) -> &Path;
pub fn manifest_file_path(&self) -> &Path;
```

New loading order:

1. Discover `.east/` (walk up from CWD).
2. Load config from `.east/config.toml`. Extract `[manifest]`.
3. Compute `manifest_repo_path` and `manifest_file_path`.
4. Load manifest from `manifest_file_path`.
5. Load state from `.east/state.toml`.

Error messages for Phase 1/2 incompatibility must be clear and actionable.

## 7. `east update` Behavior

- Does NOT fetch/checkout the manifest repo itself.
- Reads manifest from current checkout (honors uncommitted local changes).
- User manages manifest repo via plain git.

## 8. Error Model

| Error | Description |
|---|---|
| `ConfigError::ManifestSectionMissing` | Phase 1/2 workspace detected, upgrade hint |
| `ConfigError::InvalidManifestPath` | Absolute, empty, or contains `..` |
| `WorkspaceError::ManifestFileNotFound` | Manifest file missing at computed path |
| `WorkspaceError::AlreadyInitialized` | `.east/` exists without `--force` |

`ManifestError::SelfPathMismatch` is a **warning** via `tracing::warn!`, not a hard error.

## 9. Non-Goals

- No Phase 3 features (build, runner, state.toml schema changes).
- No automatic manifest repo updating.
- No migration tool for Phase 1/2 workspaces.
- No submodule support, no multi-manifest.
- No `manifest.revision` tracking.
