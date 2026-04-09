# Phase 1 开发记录

## 交付内容

Phase 1 所有目标均已达成：

- `east init <manifest>` — 从本地路径或 git URL 初始化工作空间
- `east update` — 并发 fetch/checkout，带 indicatif 进度条（8 个并发操作）
- `east list` — 以表格形式列出项目及其克隆状态
- `east status` — 每个项目的 dirty/clean 状态、HEAD SHA、分支名
- `east manifest --resolve` — 打印完全解析后的 manifest（包含传递性 imports）

## Crate 概览

| Crate | 代码行数 | 测试数 | 关键设计决策 |
|---|---|---|---|
| `east-manifest` | ~350 | 41 | BFS import 解析；`glob-match` 做 allowlist；先定义者优先 |
| `east-vcs` | ~150 | 8 | 通过 `tokio::process` 异步 shell-out；不用 libgit2 |
| `east-workspace` | ~80 | 7 | 向上查找式 workspace 发现，参照 git 的 `GIT_DIR`；使用 canonicalize 路径 |
| `east-cli` | ~280 | 9 | `clap` derive；`tokio` 运行时；`indicatif` 进度条；`Semaphore` 控制并发 |

**总计：65 个测试，全部通过。**

## 顺利的部分

1. **TDD 纪律回报明显。** 先写测试的方式提前发现了多个 serde 问题（缺少 `#[serde(default)]`、kebab-case 重命名、`skip_serializing_if`）。

2. **Manifest 解析很直观。** BFS 配合 visited 集合非常干净。使用 `std::fs::canonicalize` 作为 visited key 正确处理了符号链接。

3. **Crate 边界设计合理。** `east-manifest`、`east-vcs`、`east-workspace` 彼此无依赖。CLI 是唯一将它们组装在一起的 crate。

4. **`tokio` + `Semaphore` 并发更新实现简单。** 每个项目一个 task，有界信号量（8 并发）兼顾了吞吐量和简洁性。

## 困难的部分

1. **Rust 1.82.0 与 edition 2024 依赖冲突。** 多个 crate 更新（tempfile 3.20+、clap 4.6+、assert_cmd 2.2+）使用了 1.82.0 Cargo 不支持的 edition 特性，需要为 dev-dependencies 设置版本上限。

2. **Clippy pedantic + nursery。** Pedantic clippy 发现了真实问题（`#[must_use]`、`doc_markdown` 反引号），但也需要仔细标注。`module_name_repetitions` lint 需要在错误枚举上加 `#[allow]`（模块名为 `error`，类型为 `ManifestError` 的情况）。

3. **测试环境中的 git commit 签名。** 创建 git fixture 仓库的测试辅助函数需要设置 `commit.gpgsign = false`，以适配有签名 hook 的 CI/开发环境。

4. **Group filter 语义。** 确定无 group 项目（始终包含）与有 group 但不匹配任何 filter 的项目的确切语义需要仔细思考。当前规则：无 group 的项目始终被包含；有 group 的项目需要至少一个 `+` 匹配且无 `-` 匹配。

## Phase 中做出的决策

1. **`serde_yaml` 同时用于解析和输出。** `manifest --resolve` 使用 `serde_yaml` 序列化输出，而非单独的序列化器。这意味着输出不会保留原始文件的注释或格式，但语义上是正确的。

2. **`east init` 将 manifest 仓库克隆到临时目录。** 当给定 git URL 时，init 克隆到临时目录，提取 `east.yml`，然后丢弃克隆。这避免了在工作空间中保留单独的「manifest 仓库」目录（与 west 的 T2 拓扑不同）。

3. **Phase 1 没有实现 `state.toml`。** 设计文档提到了 `.east/state.toml`，但 Phase 1 实际上不需要它。`.east/` 目录本身就足以作为工作空间标记。状态跟踪会在需要时添加。

4. **Import 解析使用 BFS 而非 DFS。** BFS 给出更直观的顺序（广度优先合并项目），且循环检测通过 visited 集合自然正确。

## Phase 2 展望

- **配置系统**（`east-config`）：三层 TOML 配置（workspace、用户、项目）
- **扩展命令**（`east-command`）：manifest 中的 `commands:` 部分，模板变量展开
- **模板变量** 扩展至 `${workspace.root}` 之外：`${project.*}`、`${config.*}`、`${env.*}`
