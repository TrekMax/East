# east

面向 MCU/SoC 开发的、SDK 无关的、用 Rust 编写的快速多仓库与工具链前端。

[English](README.md)

## 概述

`east` 是一个通用框架，服务于任何需要以下能力的 MCU SDK：

1. **多仓库管理** — manifest 驱动，支持并发 fetch
2. **扩展命令机制** — 在 manifest 中定义自定义命令
3. **分层配置系统** — workspace、用户与项目级别的 TOML 配置
4. **可插拔的 runner 抽象** — 通过 OpenOCD、串口 ISP 等进行 flash / debug / attach / reset

灵感来源于 Zephyr 的 `west`，但**刻意不是** `west` 的克隆。`east` 与 SDK 无关，主要面向 RISC-V MCU。

## 状态

**Phase 2** — 已完成。

- **Phase 1：** 多仓库管理 — `east init`、`east update`、`east list`、`east status`、`east manifest --resolve`
- **Phase 2：** 配置与扩展命令 — `east config get/set/unset/list`、manifest 声明命令（`exec`/`script`/`executable`）、PATH 上的 `east-<name>` 发现、模板引擎

## 快速开始

```bash
# 从 manifest 仓库初始化工作空间
east init https://github.com/your-org/sdk-manifest

# 更新所有项目
east update

# 运行 manifest 中声明的命令
east hello

# 配置
east config set user.name trekmax
east config get user.name
```

## 安装

### 从 Git 安装（推荐）

```bash
cargo install --git https://github.com/TrekMax/East east-cli
```

### 从源码安装

```bash
git clone https://github.com/TrekMax/East.git
cd East
cargo install --path crates/east-cli
```

### 仅构建（不安装）

```bash
cargo build --release
# 二进制文件：target/release/east
```

输出为单一静态二进制文件，不依赖 Python 运行时。

## 支持的平台

- Linux（x86_64、aarch64）
- macOS（universal）
- Windows（x86_64）

## 许可证

本项目采用以下任一许可证：

- Apache License, Version 2.0（[LICENSE-APACHE](LICENSE-APACHE) 或 <http://www.apache.org/licenses/LICENSE-2.0>）
- MIT License（[LICENSE-MIT](LICENSE-MIT) 或 <http://opensource.org/licenses/MIT>）

由您选择。
