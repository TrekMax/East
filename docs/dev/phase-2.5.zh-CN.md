# Phase 2.5 开发记录

## 交付内容

四项债务全部解决：

1. **Script 路径解析修复** — `script:` 字段现在相对于声明它的 manifest 文件解析，而非 workspace 根。新增 `CommandDecl` 的 `declared_in` 字段（在 resolve 时填充）和 `ManifestRelativePath` 辅助类型。

2. **miette 集成** — 从 `east-cli` 中移除 `anyhow`，全面替换为 `miette::Result`。所有 library 错误类型（`ManifestError`、`ConfigError`、`TemplateError`、`CommandError`、`PathResolveError`）都 derive 了 `miette::Diagnostic`。启用了终端富文本渲染。

3. **依赖整合** — 所有共享依赖移至 `[workspace.dependencies]`，每个版本 pin 都有行内注释说明原因。策略记录在 `docs/dev/dependency-policy.md`。

4. **ManifestRelativePath** — `east-manifest::path_resolve` 中的可复用路径解析器，Phase 3 的 runner 配置路径可直接使用。

## 测试概览

| 新增测试 | 描述 |
|---|---|
| 5 | `ManifestRelativePath` 解析逻辑（相对、绝对、父级逃逸、缺失、深层嵌套） |
| 2 | Script 路径解析（导入 manifest、回归测试） |
| **总计：130 个测试**（Phase 2 的 123 + 7 个新增） |

## 顺利的部分

1. **`ManifestRelativePath` 很干净。** 简单结构体、简单方法、充分的边界情况测试。Phase 3 的 runner 配置可直接复用。

2. **miette 迁移是机械性的。** `anyhow::Result` → `miette::Result`，`.context()` → `.into_diagnostic().wrap_err()`。替换后所有测试立即通过。

3. **通过 resolve() 传播 `declared_in`。** 在 BFS resolve 过程中给 `CommandDecl` 打上声明 manifest 路径的标记很自然——那个时刻路径已知。

## 困难的部分

1. **miette 的 `.context()` 与 anyhow 的区别。** miette 的 `Context` trait 要求错误类型实现 `Diagnostic`，而 anyhow 接受任何 `Display`。需要在非 Diagnostic 错误上先 `.into_diagnostic()` 再 `.wrap_err()`。

2. **重复 crate 版本。** `miette 7.4` 依赖 `thiserror 1.x`，而我们使用 `thiserror 2.x`。还有 `unicode-width` 0.1 vs 0.2 的分裂。都是不可避免的传递性冲突——已记录在开发记录中。

## 做出的决策

1. **在 `dispatch_manifest_command` 上 `#[allow(clippy::too_many_lines)]`。** 添加 `ManifestRelativePath` 解析后函数增长到 103 行。选择 allow 而非拆分，因为该函数是无共享状态的直线分发。

2. **miette 版本 pin 为 `<7.5`。** 预防性措施——未来 miette 版本可能引入 edition 2024 的传递性依赖。MSRV 升级时放宽。

## cargo tree --duplicates 输出

```
thiserror v1（来自 miette）vs v2（我们的代码）— 不可避免
unicode-width v0.1（来自 miette）vs v0.2（来自 indicatif）— 不可避免
```
