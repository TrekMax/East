# Phase 3 设计文档 — 构建、烧录与调试

**状态：** 生效中
**范围：** `east build`、`east flash`、`east debug`、`east attach`、`east reset`；`east-build` crate、`east-runner` crate、`.east/state.toml`。

## 1. 目标

交付第一次端到端「构建并在真实硬件上运行固件」的体验：

1. **`east-build` crate** — CMake 包装，支持 preset、pristine 检测、构建产物追踪。
2. **`east-runner` crate** — Runner trait，两种 runner 类型（External、OpenOCD），`DebugSession` RAII guard。
3. **`.east/state.toml`** — 持久化 workspace 状态，追踪 build 产物、默认 runner 与用户偏好。

**硬验收标准：** 作者必须在真实 RISC-V 开发板上运行 `east build -p my-preset && east flash && east debug`，并在 gdb 中命中断点。

### 显式非目标

- 无串口 ISP 协议 / `SerialRunner`（Phase 4）。
- 无 `probe-rs` 集成。
- 无 GDB 前端 / IDE 集成。
- 无 CMake 以外的构建系统。
- 无 sysbuild / 多镜像构建。
- 无自动端口 / 设备发现。
- 无 `east run` 或 `east test`。

## 2. `east build` 语义

### 2.1 CMake 版本

最低：**3.21**（Preset schema v3）。CLI 启动时检测；版本过低硬错误。

### 2.2 构建目录

默认：`<workspace_root>/build/<name>`，`<name>` 为 preset 名（`-p/--preset`）或 `default`。覆盖：`-d/--build-dir <path>`。

### 2.3 Source 目录优先级

1. `--source-dir` CLI flag。
2. config 中的 `build.source_dir`。
3. `<workspace_root>/app`（若存在）。
4. `<workspace_root>`。

### 2.4 Pristine 策略

| 策略 | 行为 |
|---|---|
| `always` | configure 前删除 build 目录。 |
| `never` | 从不删除。 |
| `auto`（默认） | source 目录变了、preset 变了、toolchain 文件变了或 `CMakeCache.txt` 不可读时删除。 |

### 2.5 参数透传

- 第一个 `--`：追加到 configure 阶段（`east build -- -DFOO=bar`）。
- 第二个 `--`：追加到 build 阶段（`east build -- -DFOO=bar -- -v -j4`）。

### 2.6 成功判据

`cmake --build` 以 0 退出**且**在 build 目录下找到至少一个匹配 `**/*.elf`、`**/*.bin` 或 `**/*.hex` 的产物。未找到 = 警告（非错误）。可通过 `build.elf_pattern` config 覆盖。

### 2.7 State 更新

成功时写入 `.east/state.toml`：

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

失败时不动 state.toml。

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
    // Serial 保留给 Phase 4
}

pub struct RunnerCapabilities {
    pub flash: bool,
    pub debug: bool,
    pub attach: bool,
    pub reset: bool,
    pub erase: bool, // Phase 3 中始终为 false
}
```

**关键决策：**

- `capabilities()` 同步。CLI 在 dispatch 之前检查并给出干净错误。
- `debug()`/`attach()` 返回 `DebugSession`（持有 OpenOCD 子进程的 RAII guard）。
- `DebugSession` Drop：SIGTERM → 等 2 秒 → SIGKILL（Unix）；`taskkill /T /F`（Windows）。
- `RunCtx` 由 CLI 从 state.toml 和 manifest 填充。Runner 不自己搜产物。

## 4. OpenOCD Runner

### 4.1 命令

| 操作 | 命令 |
|---|---|
| flash | `openocd -f <cfg> -c "program <artifact> verify reset exit"` |
| reset | `openocd -f <cfg> -c "init; reset; exit"` |
| debug | 后台：`openocd -f <cfg>`，前台：`<gdb> <elf> -ex "target remote :<port>" -ex "load"` |
| attach | 同 debug 但 gdb 省略 `load` |

Flash 产物优先级：elf > hex > bin（取第一个存在的）。

### 4.2 Config 路径

使用 Phase 2.5 的 `ManifestRelativePath`。`runners:` 声明携带 `declared_in`，同 `CommandDecl`。

### 4.3 二进制解析

**gdb：** `--gdb` flag > `runner.openocd.gdb` config > `runner.<name>.gdb` config > `riscv64-unknown-elf-gdb` PATH。

**OpenOCD：** 同样模式，默认 `openocd`。

### 4.4 端口默认

gdb：3333，telnet：4444，TCL：6666。可按 runner 通过 `runner.<name>.gdb_port` 覆盖。

## 5. OpenOCD 输出分类

OpenOCD 几乎把所有东西写到 stderr，包括成功。**绝不能**用「stderr 非空」作为失败信号。

- 用 `tokio::io::BufReader` 逐行捕获两个流。
- 对每行分类：
  - **Ready：** 匹配 `Listening on port \d+ for gdb connections`。
  - **Error：** 匹配 `^Error:` 或流末含 `failed` 且 exit code 非零。
  - **Progress：** 其他（以 `tracing::debug!` 记录）。
- 失败时错误包含最后 20 行输出。
- 在 `OpenOcdOutputClassifier` 模块中实现，用真实硬件捕获的 fixture 做单元测试。

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

- 每个能力（`flash`/`reset`/`debug`/`attach`）是可选的带 `command` 字段的对象。
- 命令由模板引擎渲染，使用 `runner.*` 命名空间。
- Shell 执行复用 Phase 2 `exec:` 命令代码路径（`sh -c` / `cmd /C`）。

## 7. 模板引擎扩展

新命名空间：`runner.*`。

| 绑定 | 描述 |
|---|---|
| `runner.elf` | elf 绝对路径（或空） |
| `runner.bin` | bin 绝对路径（或空） |
| `runner.hex` | hex 绝对路径（或空） |
| `runner.workspace_root` | workspace 根绝对路径 |
| `runner.build_dir` | state.toml 中的 build 目录 |
| `runner.name` | 当前调用的 runner 名 |

缺失 key 仍为硬错误。不得破坏已有模板测试。

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

- **位置：** `<workspace_root>/.east/state.toml`。
- **归属：** `east-workspace::state` 模块。
- **Schema 版本化：** 加载时检查；不匹配 = 错误要求用户删除并重新构建。
- **原子写：** 写到 `.east/state.toml.tmp`，然后 rename。
- **缺失文件：** 返回 `State::default()`，`schema_version = 1`，字段全空。

## 9. Manifest Runner 声明

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

**校验：**

- `name` 匹配 `[a-z][a-z0-9-]*`。
- `type` 为 `openocd` 或 `external`。`serial` 为保留值 →「保留给 Phase 4，尚未实现」错误。
- `openocd`：`config` 字段必需。
- `external`：至少有一个能力。
- Runner 名在已解析 manifest 中唯一；冲突 = 硬错误。

## 10. 错误模型

| Crate | 错误类型 | 关键变体 |
|---|---|---|
| `east-build` | `BuildError` | `CmakeNotFound`、`CmakeVersionTooLow`、`SourceDirNotFound`、`ConfigureFailed`、`BuildFailed`、`NoArtifactsFound` |
| `east-runner` | `RunnerError` | `ConfigFileNotFound`、`BinaryNotFound`、`SpawnFailed`、`NonZeroExit`、`StartupTimeout`、`ArtifactMissing`、`CapabilityUnsupported` |
| `east-workspace` | `StateError` | `SchemaVersionMismatch`、`TomlParse`、`Io` |

全部 derive `miette::Diagnostic`。

## 11. 子进程生命周期规则

- **Spawn：** 后台 OpenOCD 用 `stdin(null)`、`stdout(piped)`、`stderr(piped)`；前台 gdb 用 `Stdio::inherit()`。
- **终止：** SIGTERM → 等 2 秒 → SIGKILL（Unix）；`taskkill /T /F`（Windows）。
- **防僵尸：** 总是 `Child::wait()`，包括 kill 路径。
- **Windows：** `CREATE_NEW_PROCESS_GROUP`；CLI Ctrl+C handler 委派给 `DebugSession`。
- **可测性：** `SubprocessHost` trait，mock 实现用于测试，生产实现用 `tokio::process`。

## 12. Crate 依赖图

```
east-cli ─┬─► east-command ─► east-manifest
          ├─► east-config
          ├─► east-workspace ─► east-manifest
          ├─► east-vcs
          ├─► east-build    ─► east-workspace, east-config
          └─► east-runner   ─► east-manifest, east-workspace, east-config, east-command
```

**规则：** `east-runner` **不得**依赖 `east-build`。Runner 接收路径，不关心构建语义。

## 13. 测试策略

- **CMake fixture：** `tests/fixtures/phase3/hello-cmake/` — 用宿主编译器产出 `.elf` 的小型工程。
- **Fake OpenOCD：** 脚本变体：`success-flash`、`success-listen`、`error-cfg-missing`、`slow-start`。
- **Fake gdb：** 连接、打印 banner、退出 0。
- **OpenOCD 输出 fixture：** 从真实硬件捕获，每个场景一个文件。
- **CI 无法测试真实硬件。** 手工验证必需，硬件日志记录在 dev notes 中。

## 14. 性能基线

- `east --version` < 30 ms（惰性加载 build/runner）。
- `east build` 额外开销 < 50 ms。
- 任何测试路径后无僵尸进程。
- 零 `unsafe` 延续。
