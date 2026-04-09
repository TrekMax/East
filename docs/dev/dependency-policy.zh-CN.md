# 依赖版本锁定策略

## 最低支持 Rust 版本（MSRV）

MSRV 锁定在 `rust-toolchain.toml` 中（当前为 **1.85.0**）。
升级 MSRV 需要编写设计说明并在开发日志中记录。

## 规则

### 1. 锁定具有传递性 edition 要求的依赖

如果某个依赖的传递闭包要求超出我们 MSRV 的 Rust edition，则必须在
`Cargo.toml` 中设置版本上界。每条此类条目**必须**附带行内注释说明原因。

示例：

```toml
# Pinned below X.Y: some-dep requires edition beyond our MSRV
some-dep = ">=A.B, <X.Y"
```

### 2. 工作区级别依赖声明

共享依赖在根 `Cargo.toml` 的 `[workspace.dependencies]` 中声明。
成员 crate 通过 `workspace = true` 引用：

```toml
# 成员 Cargo.toml 中
[dependencies]
clap = { workspace = true }
```

### 3. 手动执行 `cargo update`

`cargo update` 手动执行，不设定计划任务。每次更新单独一个提交。
如有非平凡变更，需在开发日志中记录。

### 4. `cargo deny` 检查

`cargo deny` 配置位于 `deny.toml`（已纳入版本管理）。检查内容包括：

- **重复版本检测** — 警告级别
- **许可证白名单** — 仅允许已批准的许可证
- **Git 来源黑名单** — 发布版本中不允许 git 依赖

目前仅为信息性检查，将在 Phase 5 升级为 CI 硬性门禁。

## 当前锁定的依赖

无。工具链升级到 1.85.0 后，所有与 edition 2024 相关的版本上界约束均已移除，
因为 1.85.0 原生支持 edition 2024。

## 更新本文档

当新增、移除或调整锁定条目时，请更新上方表格并在开发日志中记录变更。
