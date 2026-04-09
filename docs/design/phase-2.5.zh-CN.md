# Phase 2.5 设计文档 — 技术债务清理

**状态：** 生效中
**范围：** Phase 3 之前的四项针对性债务清理。无新功能、无新 crate、无新子命令。

## 1. 目标

偿还 Phase 1 和 Phase 2 期间积累的四项技术债务，使 Phase 3（build/flash/debug）在干净的基础上启动。这是一个小型间奏，不是功能阶段。

### 债务项

1. **Script 路径解析 bug 修复** — `script:` 字段在某些情况下解析不正确。
2. **miette 集成** — 结构化诊断错误在 CLI 中替代 `anyhow`。
3. **依赖版本锁定策略** — 正式化并整合版本锁定。
4. **ManifestRelativePath 辅助类型** — 将路径解析提取为可复用、可测试的类型。

### 显式非目标

- 不引入 `state.toml` 或任何新的持久化状态文件。
- 不向 workspace 添加新 crate。
- 不添加新子命令。
- 不做无关重构（本文档未列出的内容一律等待）。
- Phase 3 功能（`east build`、`east flash`、`east debug`、`east attach`、`east reset`）。

### Crate 依赖图

Crate 依赖图与 Phase 2 **完全相同**：

```
east-cli  ─────────────┬─► east-command ─► east-manifest
                       │                │
                       ├─► east-config ◄┘
                       ├─► east-workspace
                       └─► east-vcs
```

无新边、无新 crate。

## 2. Script 路径解析 Bug 修复

### 2.1 规则

manifest 声明命令中的 `script:` 字段**相对于声明该命令的 manifest 文件**解析，而非相对于 workspace 根目录。这是 Phase 2（第 3.4 节）的既定意图，但当前实现未将声明 manifest 的路径传播到解析点。

### 2.2 实现

在 manifest 加载时向 `CommandDecl` 传播 `declared_in: PathBuf` 字段。该路径指向包含 `commands:` 条目的 manifest 文件（例如 `east.yml` 或导入的子 manifest）。

### 2.3 API 变更

```rust
impl CommandDecl {
    /// 返回声明此命令的 manifest 文件路径。
    pub fn declared_in(&self) -> &Path;
}
```

这是一项**增量**变更。不会移除或以破坏性方式修改任何现有公开 API。

### 2.4 边界情况

| 情况 | 行为 |
|---|---|
| `script:` 中使用绝对路径 | 直接使用；`declared_in` 不参与。 |
| `../` 逃逸（例如 `../shared/run.sh`） | 允许。拼接后的路径会被规范化。 |
| 解析后的路径不存在 | 错误同时指明 manifest 文件和尝试解析的路径（例如："script `../shared/run.sh` declared in `/repo/sub/east.yml` resolved to `/repo/shared/run.sh` which does not exist"）。 |

## 3. miette 集成

### 3.1 引入 miette 的范围

| 错误来源 | Crate | 备注 |
|---|---|---|
| Manifest 解析 / 校验错误 | `east-manifest` | YAML 语法、schema 违规、保留名冲突。 |
| Config TOML 解析错误 | `east-config` | 格式错误的 TOML、读取时的类型不匹配。 |
| 模板引擎错误 | `east-command` | 未知命名空间、缺失 key、未闭合变量。 |

### 3.2 暂不引入 miette 的范围（延后）

- `east-vcs` — git/VCS 错误目前保持不透明包装。
- `east-workspace` — workspace I/O 错误保持简单的 `thiserror` 枚举。

如果更丰富的诊断被证明有价值，将在未来阶段重新评估。

### 3.3 库边界

- **库 crate**（`east-manifest`、`east-config`、`east-command`）：使用 `thiserror` 和 miette 的 `derive(Diagnostic)`。错误类型实现 `miette::Diagnostic`，使结构化字段（code、help、labels、source snippets）可供任何消费者使用。
- **CLI crate**（`east-cli`）：使用 `miette::Report` 作为顶层错误类型，**完全替代 `anyhow`**。`anyhow` 从 `east-cli` 的 `Cargo.toml` 中移除。

### 3.4 移除 anyhow

从 `east-cli` 中移除 `anyhow`。库 crate 从未依赖 `anyhow`，因此不涉及其他 `Cargo.toml` 的变更。`main()` 返回类型变为 `miette::Result<()>`。

## 4. 依赖版本锁定策略

### 4.1 策略文件

在 `docs/dev/dependency-policy.md` 新建策略文档（中英双语）。将下述规则成文，为贡献者提供唯一参考。

### 4.2 规则

| 规则 | 详情 |
|---|---|
| MSRV | 锁定在 `rust-toolchain.toml` 中。所有 CI 任务使用此工具链。 |
| 上界约束 | 为已知在 Rust edition 边界上冲突的依赖添加上界并附注释（例如某 crate 要求 edition 2024 而我们的 MSRV 尚不支持）。 |
| Workspace 整合 | 所有版本锁定集中在根 `Cargo.toml` 的 `[workspace.dependencies]` 中。各 crate 级别的 `Cargo.toml` 通过 `workspace = true` 引用。 |
| Cargo update | 仅手动执行。不使用 Dependabot 或 Renovate 自动化。`cargo update` 需有意执行并提交 lockfile。 |
| Cargo deny | 在 workspace 根目录维护 `deny.toml` 配置。CI 运行 `cargo deny check` 以强制执行许可证和安全公告策略。 |

### 4.3 操作

将所有现有版本锁定整合至 `[workspace.dependencies]`，对非显而易见的锁定附加说明性注释。示例：

```toml
[workspace.dependencies]
serde = "1.0"
toml = "0.8"            # 锁定低于 0.9：0.9 要求 edition 2024
miette = "7"
thiserror = "2"
```

## 5. ManifestRelativePath 辅助类型

### 5.1 位置

Crate：`east-manifest`，内部模块 `path_resolve`（不在 crate 根公开；通过 `pub(crate)` 或窄范围 re-export 访问）。

### 5.2 类型定义

```rust
/// manifest 中声明的原始路径字符串，连同声明该路径的 manifest 文件。
/// 相对路径基于 manifest 所在目录解析。
pub struct ManifestRelativePath {
    /// 包含此声明的 manifest 文件的绝对路径。
    manifest_path: PathBuf,
    /// manifest 中原样书写的字符串（例如 "scripts/hello.sh"）。
    raw: String,
}
```

### 5.3 解析方法

```rust
impl ManifestRelativePath {
    /// 将原始路径解析为绝对的、规范化的路径。
    ///
    /// - 若 `raw` 是绝对路径，直接使用并规范化。
    /// - 若 `raw` 是相对路径，拼接到 `manifest_path` 的父目录上，
    ///   然后规范化。
    /// - 如果规范化失败（例如文件不存在），返回
    ///   `Err(PathResolveError)`。
    pub fn resolve(&self) -> Result<PathBuf, PathResolveError>;
}
```

`PathResolveError` 包含：

- manifest 路径。
- 原始字符串。
- 尝试解析的路径（规范化失败前的路径）。
- 底层 `io::Error`。

### 5.4 Phase 2.5 使用范围

在本阶段中，`ManifestRelativePath` **仅**用于 `CommandDecl` 的 `script:` 解析。

### 5.5 Phase 3 前瞻

Runner 配置路径（例如 manifest 中声明的 OpenOCD 配置文件路径）将在 Phase 3 中复用 `ManifestRelativePath`。**不要在 Phase 2.5 中实现 runner 路径解析。** 该类型在设计时已考虑未来用途，但本阶段唯一的调用点是 script 解析。

## 6. 测试策略

### 6.1 Script 路径解析

- 使用**真实临时目录**（通过 `tempfile` crate）并实际创建文件。
- 测试矩阵：相对路径、绝对路径、`../` 逃逸、缺失文件。
- 验证错误信息同时包含 manifest 路径和解析后的路径。

### 6.2 miette 诊断

- 对 **`Diagnostic` 结构化字段**（error code、help 文本、labels）进行断言，而非对渲染后的 ANSI 输出进行断言。
- 示例：断言 manifest 解析错误的 `code()` 返回 `Some("east-manifest::parse")`，`help()` 包含可操作的文本。
- **不要**对彩色终端输出做快照测试；这在不同 miette 版本和终端宽度下非常脆弱。

### 6.3 依赖整合

- 整合完成后运行 `cargo tree --duplicates`，断言我们控制的 crate 零重复。
- CI 使用新的 `deny.toml` 运行 `cargo deny check`。

### 6.4 ManifestRelativePath

- 与 script 路径测试相同的真实临时目录方法。
- 在 `east-manifest::path_resolve` 中编写单元测试，覆盖：相对路径解析、绝对路径直通、`../` 规范化、缺失文件错误内容。
