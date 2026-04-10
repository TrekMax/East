# Phase 3 Design Document — Build, Flash & Debug

**Status:** Active
**Scope:** `east build`, `east flash`, `east debug`, `east attach`, `east reset`; `east-build` crate, `east-runner` crate, `.east/state.toml`.

## 1. Goal

Deliver the first end-to-end "build and run firmware on real hardware" experience:

1. **`east-build` crate** — CMake wrapper with preset support, pristine detection, build artifact tracking.
2. **`east-runner` crate** — Runner trait, two runner kinds (External, OpenOCD), `DebugSession` RAII guard.
3. **`.east/state.toml`** — persistent workspace state tracking build products, default runner, and preferences.

**Hard acceptance criterion:** the author must run `east build -p my-preset && east flash && east debug` on a real RISC-V dev board and hit a breakpoint in gdb.

### Explicit Non-Goals

- No serial ISP protocols / `SerialRunner` (Phase 4).
- No `probe-rs` integration.
- No GDB frontend / IDE integration.
- No non-CMake build systems.
- No sysbuild / multi-image builds.
- No automatic port / device discovery.
- No `east run` or `east test`.

## 2. `east build` Semantics

### 2.1 CMake Version

Minimum: **3.21** (Preset schema v3). Detected at CLI startup; lower versions produce a hard error.

### 2.2 Build Directory

Default: `<workspace_root>/build/<name>`, where `<name>` is the preset name (from `-p/--preset`) or `default`. Override: `-d/--build-dir <path>`.

### 2.3 Source Directory Precedence

1. `--source-dir` CLI flag.
2. `build.source_dir` in config.
3. `<workspace_root>/app` if that directory exists.
4. `<workspace_root>`.

### 2.4 Pristine Strategies

| Strategy | Behavior |
|---|---|
| `always` | Remove build dir before configure. |
| `never` | Never remove. |
| `auto` (default) | Remove if: source dir changed, preset changed, toolchain file changed, or `CMakeCache.txt` is unreadable. |

### 2.5 Argument Passthrough

- First `--`: appended to configure step (`east build -- -DFOO=bar`).
- Second `--`: appended to build step (`east build -- -DFOO=bar -- -v -j4`).

### 2.6 Success Criteria

`cmake --build` exits 0 AND at least one artifact matching `**/*.elf`, `**/*.bin`, or `**/*.hex` is found under the build directory. No artifact = warning (not error). Override pattern via `build.elf_pattern` config.

### 2.7 State Update

On success, write to `.east/state.toml`:

```toml
[build]
last_build_dir = "build/default"
last_preset = "default"
last_source_dir = "/abs/path/to/source"
last_elf = "build/default/app/firmware.elf"
last_bin = "build/default/app/firmware.bin"
last_hex = ""
last_configured_at = "2026-04-09T12:34:56Z"
```

On failure, state.toml is not touched.

## 3. Runner Trait

```rust
#[async_trait]
pub trait Runner: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> RunnerKind;
    fn capabilities(&self) -> RunnerCapabilities;
    async fn flash(&self, ctx: &RunCtx, opts: &FlashOpts) -> Result<(), RunnerError>;
    async fn debug(&self, ctx: &RunCtx, opts: &DebugOpts) -> Result<DebugSession, RunnerError>;
    async fn attach(&self, ctx: &RunCtx, opts: &AttachOpts) -> Result<DebugSession, RunnerError>;
    async fn reset(&self, ctx: &RunCtx, opts: &ResetOpts) -> Result<(), RunnerError>;
}

pub enum RunnerKind {
    External,
    OpenOcd,
    // Serial reserved for Phase 4
}

pub struct RunnerCapabilities {
    pub flash: bool,
    pub debug: bool,
    pub attach: bool,
    pub reset: bool,
    pub erase: bool, // always false in Phase 3
}
```

**Key decisions:**

- `capabilities()` is synchronous. CLI checks before dispatch with a clean error message.
- `debug()`/`attach()` return `DebugSession` (RAII guard holding OpenOCD child process).
- `DebugSession` Drop: SIGTERM → 2s wait → SIGKILL (Unix); `taskkill /T /F` (Windows).
- `RunCtx` is populated by CLI from state.toml and manifest. Runners do not search for artifacts.

```rust
pub struct RunCtx<'a> {
    pub workspace: &'a Workspace,
    pub manifest: &'a Manifest,
    pub config: &'a Config,
    pub state: &'a State,
    pub elf_path: Option<&'a Path>,
    pub bin_path: Option<&'a Path>,
    pub hex_path: Option<&'a Path>,
    pub openocd_binary: &'a Path,
    pub gdb_binary: &'a Path,
}
```

## 4. OpenOCD Runner

### 4.1 Commands

| Operation | Command |
|---|---|
| flash | `openocd -f <cfg> -c "program <artifact> verify reset exit"` |
| reset | `openocd -f <cfg> -c "init; reset; exit"` |
| debug | Background: `openocd -f <cfg>`, then foreground: `<gdb> <elf> -ex "target remote :<port>" -ex "load"` |
| attach | Same as debug but gdb omits `load` |

Flash artifact precedence: elf > hex > bin (first present wins).

### 4.2 Config Path

Uses `ManifestRelativePath` from Phase 2.5. The `runners:` declaration carries `declared_in`, same as `CommandDecl`.

### 4.3 Binary Resolution

**gdb:** `--gdb` CLI flag > `runner.openocd.gdb` config > `runner.<name>.gdb` config > `riscv64-unknown-elf-gdb` PATH lookup.

**OpenOCD:** Same pattern with `runner.openocd.binary`, default `openocd`.

### 4.4 Port Defaults

gdb: 3333, telnet: 4444, TCL: 6666. Configurable per runner via `runner.<name>.gdb_port` etc.

## 5. OpenOCD Output Classification

OpenOCD writes almost everything to stderr, including success. **Never** use "stderr non-empty" as a failure signal.

- Capture both streams line-by-line via `tokio::io::BufReader`.
- Classify each line:
  - **Ready:** matches `Listening on port \d+ for gdb connections`.
  - **Error:** matches `^Error:` or contains `failed` at end with non-zero exit.
  - **Progress:** everything else (logged at `tracing::debug!`).
- On failure, include last 20 lines of captured output in the error.
- Implemented in `OpenOcdOutputClassifier` module with unit tests on real captured fixtures.

## 6. External Runner

```yaml
runners:
  - name: custom-tool
    type: external
    flash:
      command: "my-flasher --elf ${runner.elf} --port ${config.runner.custom-tool.port}"
    reset:
      command: "my-flasher --reset"
```

- Each capability (`flash`/`reset`/`debug`/`attach`) is an optional object with a `command` field.
- Command rendered by template engine with `runner.*` namespace.
- Shell execution reuses Phase 2 `exec:` code path (`sh -c` / `cmd /C`).

## 7. Template Engine Extension

New namespace: `runner.*`.

| Binding | Description |
|---|---|
| `runner.elf` | Absolute path to elf (or empty) |
| `runner.bin` | Absolute path to bin (or empty) |
| `runner.hex` | Absolute path to hex (or empty) |
| `runner.workspace_root` | Workspace root absolute path |
| `runner.build_dir` | Build directory from state.toml |
| `runner.name` | Name of the invoked runner |

Missing key remains a hard error. Must not break existing template tests.

## 8. `.east/state.toml` Schema v1

```toml
schema_version = 1

[build]
last_build_dir = "build/default"
last_preset = "default"
last_source_dir = "/abs/path/to/source"
last_elf = "build/default/app/firmware.elf"
last_bin = ""
last_hex = ""
last_configured_at = "2026-04-09T12:34:56Z"

[runner]
default = "wch-link"
```

- **Location:** `<workspace_root>/.east/state.toml`.
- **Ownership:** `east-workspace::state` module.
- **Schema versioning:** checked on load; mismatch = error asking user to delete and rebuild.
- **Atomic writes:** write to `.east/state.toml.tmp`, then rename.
- **Missing file:** return `State::default()` with `schema_version = 1` and empty fields.

## 9. Manifest Runner Declaration

```yaml
runners:
  - name: wch-link
    type: openocd
    config: openocd/wch-riscv.cfg
    gdb_port: 3333
  - name: custom-tool
    type: external
    flash:
      command: "..."
```

**Validation:**

- `name` matches `[a-z][a-z0-9-]*`.
- `type` is `openocd` or `external`. `serial` is reserved → "not yet implemented, reserved for Phase 4" error.
- `openocd`: `config` field required.
- `external`: at least one capability present.
- Runner names unique in resolved manifest; collision = hard error.

## 10. Error Model

| Crate | Error Type | Key Variants |
|---|---|---|
| `east-build` | `BuildError` | `CmakeNotFound`, `CmakeVersionTooLow`, `SourceDirNotFound`, `ConfigureFailed`, `BuildFailed`, `NoArtifactsFound` |
| `east-runner` | `RunnerError` | `ConfigFileNotFound`, `BinaryNotFound`, `SpawnFailed`, `NonZeroExit`, `StartupTimeout`, `ArtifactMissing`, `CapabilityUnsupported` |
| `east-workspace` | `StateError` | `SchemaVersionMismatch`, `TomlParse`, `Io` |

All derive `miette::Diagnostic`.

## 11. Subprocess Lifecycle Rules

- **Spawn:** `tokio::process::Command` with `stdin(null)`, `stdout(piped)`, `stderr(piped)` for background OpenOCD; `Stdio::inherit()` for foreground gdb.
- **Termination:** SIGTERM → 2s wait → SIGKILL (Unix); `taskkill /T /F` (Windows).
- **Zombie prevention:** always `Child::wait()`, including kill paths.
- **Windows:** `CREATE_NEW_PROCESS_GROUP` flag; CLI Ctrl+C handler delegates to `DebugSession`.
- **Testability:** `SubprocessHost` trait with `spawn`, `wait_ready`, `kill_gracefully`. Mock impl for tests; production uses `tokio::process`.

## 12. Crate Dependency Graph

```
east-cli ─┬─► east-command ─► east-manifest
          ├─► east-config
          ├─► east-workspace ─► east-manifest
          ├─► east-vcs
          ├─► east-build    ─► east-workspace, east-config
          └─► east-runner   ─► east-manifest, east-workspace, east-config, east-command
```

**Rule:** `east-runner` MUST NOT depend on `east-build`. Runners receive paths, not build semantics.

## 13. Testing Strategy

- **CMake fixture:** `tests/fixtures/phase3/hello-cmake/` — tiny project producing `.elf` via host compiler.
- **Fake OpenOCD:** scripts with variants: `success-flash`, `success-listen`, `error-cfg-missing`, `slow-start`.
- **Fake gdb:** script that connects, prints banner, exits 0.
- **OpenOCD output fixtures:** captured from real hardware, one file per scenario.
- **CI cannot test real hardware.** Manual validation required with hardware log in dev notes.

## 14. Performance Bars

- `east --version` < 30 ms (lazy loading of build/runner).
- `east build` overhead < 50 ms before CMake invoked.
- No zombie processes after any test path.
- Zero `unsafe` continues.
