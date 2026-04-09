# Phase 2 Design Document — Configuration & Extension Commands

**Status:** Active
**Scope:** `east config` CLI + layered TOML config; extension command discovery, dispatch, and template engine.

## 1. Goal

Deliver two orthogonal capabilities on top of the Phase 1 multi-repo core:

1. **Layered configuration system** — `east config` CLI, three-layer TOML merge (system / global / workspace), typed access from library code.
2. **Extension command mechanism** — discover and dispatch user-defined commands declared in `east.yml` (`commands:` section) or found on `PATH` as `east-<name>` executables, with a template engine for variable substitution.

### End-State User Stories

- `east config set user.name trekmax` persists to `~/.config/east/config.toml`.
- `commands: [{ name: hello, exec: "echo hi from ${workspace.root} as ${config.user.name}" }]` in `east.yml` is invocable via `east hello`.
- An `east-myext` binary on `PATH` is invocable via `east myext -- --some-flag`.

### Explicit Non-Goals

- `east build`, `east flash`, `east debug`, `east attach`, `east reset` (Phase 3).
- CMake detection, OpenOCD, probe-rs, serial runners.
- Dynamic library / WASM plugin loading.
- Config schema validation for third-party keys.
- `east config edit` (spawning `$EDITOR`).
- Tab completion for dynamically discovered commands.

## 2. Configuration System

### 2.1 Format

TOML. Not INI, YAML, or JSON.

### 2.2 File Locations

| Layer | Linux / macOS | Windows |
|---|---|---|
| System | `/etc/east/config.toml` | `%PROGRAMDATA%\east\config.toml` |
| Global | `$XDG_CONFIG_HOME/east/config.toml` (fallback: `~/.config/east/config.toml`) | `%APPDATA%\east\config.toml` |
| Workspace | `<workspace_root>/.east/config.toml` | `<workspace_root>\.east\config.toml` |

### 2.3 Merge Semantics

- **Precedence (lowest to highest):** system → global → workspace.
- Later layers override earlier layers on a per-key basis (deep merge of nested tables).
- Missing layers are silently skipped (no error if system or workspace config does not exist).

### 2.4 Key Namespacing

Dotted paths: `user.name`, `update.parallelism`, `runner.default`. Internally represented as nested TOML tables.

Unknown keys are allowed and preserved. `east` core reserves:

- `user.name`, `user.email`
- `update.parallelism` (integer, default 8)
- `runner.default` (string)
- `manifest.file` (string, default `east.yml`)

SDK extensions may use any other namespace.

### 2.5 CLI Type Handling

`east config set KEY VALUE` treats values as strings by default. Flags for typed writes:

- `--int` — parse as integer
- `--bool` — parse as boolean (`true`/`false`)
- `--float` — parse as float

Reads always return the stored type.

### 2.6 Config Path Resolution

Config file paths are resolved via an injectable `PathProvider` trait, enabling hermetic testing without touching the real filesystem. The default implementation reads platform-specific environment variables and directories.

### 2.7 Config I/O

- Synchronous. Happens before the tokio runtime starts or inside `spawn_blocking`.
- No `tokio::fs` — files are small enough that async I/O adds complexity without benefit.
- Writes use dotted-key form at the top level (`user.name = "x"`) to avoid ambiguity when merging.

## 3. Extension Command Mechanism

### 3.1 Discovery Sources (in order)

1. **Manifest-declared commands** from the resolved `east.yml` `commands:` section.
2. **PATH-based executables** matching `east-<name>` (Windows: also checks `PATHEXT` extensions).

If both sources define the same name, **manifest-declared wins** with a warning.

Built-in commands always win and cannot be shadowed:
`init`, `update`, `list`, `status`, `manifest`, `config`, `help`, `version`.

### 3.2 Reserved Command Names

These names are reserved for future built-in commands and **cannot** be declared in manifests, even if not yet implemented:

`build`, `flash`, `debug`, `attach`, `reset`, `import-west`.

A manifest declaring any of these triggers a hard validation error at load time.

### 3.3 Manifest-Declared Command Schema

```yaml
commands:
  - name: hello                       # required, [a-z][a-z0-9-]*
    help: "Say hello"                 # required, single line
    long-help: |                      # optional, multi-line
      Longer description shown by `east help hello`.
    exec: "echo hi from ${workspace.root}"   # one of exec | executable | script
    # executable: east-myext          # delegate to a PATH binary by explicit name
    # script: scripts/hello.sh        # path relative to the manifest declaring it
    args:                             # optional, declarative arg schema
      - name: target
        help: "Target name"
        required: false
        default: "world"
    env:                              # optional, extra env vars
      FOO: "bar"
    cwd: "${workspace.root}"          # optional, working directory
```

**Validation rules:**

- Exactly one of `exec`, `executable`, `script` must be present.
- `name` must match `[a-z][a-z0-9-]*`.
- `name` must not collide with built-in or reserved command names.
- Violations are manifest validation errors surfaced at `east.yml` load time.

### 3.4 Shell Execution Rules

For `exec:` commands after template rendering:

- **Unix:** `sh -c <rendered_string>`
- **Windows:** `cmd /C <rendered_string>`

For `script:` commands:

- The script path is resolved **relative to the manifest file that declared it**, not to `cwd`.
- The script is invoked directly (must be executable on Unix, or have an appropriate extension on Windows).

For `executable:` commands:

- Looked up on `PATH` by the given name.

### 3.5 Argument Passing

- **Manifest-declared args:** populate `${arg.name}`, parsed by a dynamically built clap `Command` at dispatch time.
- **Tokens after `--`:** passed through verbatim to exec/executable/script, appended after manifest-declared args.
- **PATH-based commands with no manifest declaration:** ALL tokens after the subcommand name are passed through verbatim (no arg parsing by `east`).

## 4. Template Engine

### 4.1 Syntax

`${namespace.key}` only. No filters, conditionals, or loops.

### 4.2 Namespaces

| Pattern | Description |
|---|---|
| `${workspace.root}` | Absolute path to workspace root |
| `${workspace.manifest}` | Absolute path to top-level `east.yml` |
| `${project.<name>.path}` | Absolute path to project checkout |
| `${project.<name>.revision}` | Resolved revision string |
| `${config.<dotted.key>}` | Value from merged config, as string |
| `${env.<NAME>}` | Environment variable |
| `${arg.<name>}` | Value of declared arg for current command |

### 4.3 Missing Key Behavior

Hard error. No silent empty-string substitution. Error message must identify the template source (manifest file path and line number where feasible).

### 4.4 Escaping

`$${...}` produces a literal `${...}`. Nothing else is special.

### 4.5 Implementation

Hand-written in ~80 lines. No templating crate dependency.

## 5. Crate Dependency Graph

```
east-cli  ─────────────┬─► east-command ─► east-manifest
                       │                │
                       ├─► east-config ◄┘
                       ├─► east-workspace
                       └─► east-vcs
```

Rules:

- `east-config` has zero dependency on `east-manifest`.
- `east-command` depends on both `east-config` and `east-manifest`.
- Neither depends on `east-cli`.

## 6. Error Model

| Crate | Error Type | Variants |
|---|---|---|
| `east-config` | `ConfigError` | `Io`, `TomlParse`, `TomlSerialize`, `KeyNotFound`, `TypeMismatch` |
| `east-command` | `CommandError` | `InvalidName`, `MutuallyExclusiveFields`, `ReservedName`, `TemplateError`, `SpawnFailed`, `NotFound` |

Template errors are a sub-enum within `east-command`:

| `TemplateError` variant | Meaning |
|---|---|
| `UnknownNamespace` | Namespace prefix not recognized |
| `MissingKey` | Key not found in the given namespace |
| `UnterminatedVariable` | `${` without closing `}` |
| `InvalidSyntax` | Other parse failure |

## 7. Performance Constraints

- `east --version` must remain under 30 ms. Config and command discovery are lazy.
- Config load skips missing layers silently.
- No blocking I/O on the async runtime.

## 8. Testing Strategy

- **Config dirs:** inject via `PathProvider` trait; never read real `$HOME` in tests.
- **PATH discovery:** create temp dirs with fake `east-foo` executables; prepend to `PATH` for child process only.
- **Env var isolation:** use `assert_cmd` (child process) or a global mutex for env-touching tests.
- **Windows coverage:** every new test suite must exercise at least one Windows-specific assertion.
- **Fixtures:** new command-focused manifests under `tests/fixtures/phase2/`.
