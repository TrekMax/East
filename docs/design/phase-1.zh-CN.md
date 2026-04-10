# Phase 1 设计文档 — 多仓库管理

**状态：** 生效中
**范围：** `east init` / `east update` / `east list` / `east status` / `east manifest --resolve`

## 1. 目标

交付一个可工作的多仓库管理工具，能够：

1. 解析带版本号的 `east.yml` manifest，描述 remotes、projects、imports 和 groups。
2. 从 manifest 初始化工作空间（`.east/` 目录）。
3. 并发克隆和更新项目仓库。
4. 列出项目及其状态（clean、dirty、revision 不匹配）。
5. 解析 manifest 并包含传递性 imports，带循环检测。

五个命令（`init`、`update`、`list`、`status`、`manifest --resolve`）必须能对包含 3–5 个 git 仓库的 fixture SDK manifest 正常工作。

## 2. 涉及的 Crate

| Crate | Phase 1 中的角色 |
|---|---|
| `east-manifest` | Manifest 数据模型、YAML 解析、import 解析、循环检测 |
| `east-vcs` | 通过 shell-out 调用系统 `git` 进行 Git 操作 |
| `east-workspace` | `.east/` 目录布局、workspace 发现（从 CWD 向上查找） |
| `east-cli` | `clap` 入口，将上述 crate 组装为 CLI 命令 |

Phase 1 **不涉及**的 crate：`east-config`、`east-command`、`east-runner`、`east-build`，保持空桩。

## 3. Manifest Schema（v1）

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
    path: sdk/core          # 可选；默认值为 name
    remote: origin          # 可选；回退到 defaults.remote
    revision: v1.2.0        # 可选；回退到 defaults.revision
    groups: [required]      # 可选；默认为 []
  - name: sdk-drivers
    path: sdk/drivers
    groups: [required]
  - name: sdk-examples
    groups: [optional]

imports:
  - file: sdk/core/east.yml
    allowlist: [hal-*]      # 可选 glob 过滤导入的项目名

group-filter: [+required, -optional]

commands: []   # Phase 2；Phase 1 解析时忽略
runners: []    # Phase 3；Phase 1 解析时忽略
```

### 3.1 数据模型

- **`Manifest`**：顶层结构体。字段：`version`、`remotes`、`defaults`、`projects`、`imports`、`group_filter`。
- **`Remote`**：`name`、`url_base`。
- **`Defaults`**：`remote`（可选）、`revision`（可选）。
- **`Project`**：`name`、`path`（可选，默认为 `name`）、`remote`（可选）、`revision`（可选）、`groups`（可选）。
- **`Import`**：`file`（相对路径）、`allowlist`（可选 glob 模式列表）。
- **Group filter**：`+group` / `-group` 字符串列表。项目被包含的条件：属于至少一个 `+` group 且不属于任何 `-` group。没有 group 的项目始终被包含。

### 3.2 Import 解析

Import 采用递归解析：

1. 解析顶层 manifest。
2. 对 `imports` 中的每一项，将 `file` 路径**相对于包含该 import 声明的 manifest 所在目录**进行解析。
3. 解析被导入的 manifest。
4. 通过 `allowlist` 过滤其 projects（按项目名进行 glob 匹配）。
5. 将导入的 projects 合并到已解析集合（先定义者优先；不覆盖）。
6. 递归处理被导入 manifest 自身的 `imports`。

**循环检测：** 维护一个 `HashSet<PathBuf>`，存储规范化后的绝对路径。解析任何 manifest 文件前先检查是否已访问，若已访问则报错。

### 3.3 模板变量

Phase 1 仅在字符串值中支持 `${workspace.root}`。其他命名空间（`project.*`、`config.*`、`env.*`）推迟到后续 Phase。

## 4. 工作空间布局

```
<workspace-root>/
├── .east/
│   ├── config.toml       # Phase 2；Phase 1 中创建为空文件
│   └── state.toml        # 记录 workspace 根 manifest 路径、最后更新时间
├── east.yml              # 顶层 manifest（用户提供或 init 时生成）
├── sdk/
│   ├── core/             # 已克隆的项目
│   └── drivers/          # 已克隆的项目
└── sdk-examples/         # 已克隆的项目
```

### 4.1 Workspace 发现

从 CWD 向上查找包含 `.east/` 的目录。在文件系统根或挂载边界处停止。逻辑参照 git 的 `GIT_DIR` 发现机制。

## 5. Git 操作（`east-vcs`）

所有 git 操作通过 shell-out 调用系统 `git`。不使用 `libgit2` 或 `git2-rs` 绑定。

Phase 1 所需操作：

| 操作 | 命令 |
|---|---|
| Clone | `git clone --single-branch -b <revision> <url> <path>` |
| Fetch | `git -C <path> fetch origin` |
| Checkout | `git -C <path> checkout <revision>` |
| 当前 HEAD | `git -C <path> rev-parse HEAD` |
| 当前分支 | `git -C <path> rev-parse --abbrev-ref HEAD` |
| 是否 dirty | `git -C <path> status --porcelain`（非空 = dirty） |
| 远端 URL | `git -C <path> remote get-url origin` |

### 5.1 URL 构造

项目的完整 clone URL：`<remote.url_base>/<project.name>`（如果项目指定了绝对 URL 则使用 `<project.url>` — v1 schema 中未包含但已预留）。

### 5.2 错误处理

所有 git 命令返回 `Result<Output>`，包装进程退出码、stdout 和 stderr。错误信息携带完整命令行和 stderr 以便诊断。

## 6. CLI 命令

### `east init <manifest-url-or-path> [-r <revision>]`

1. 克隆 manifest 仓库（或复制本地 manifest 文件）到当前目录。
   指定 `-r` / `--revision` 时，从指定分支或标签获取。
2. 创建 `.east/` 目录和 `state.toml`。
3. 隐式运行 `east update`。

### `east update`

1. 发现 workspace 根目录。
2. 解析并 resolve manifest（包括 imports）。
3. 应用 group filter。
4. 对每个包含的项目，并发执行：
   - 未克隆：`git clone`。
   - 已克隆：`git fetch` + `git checkout <revision>`。
5. 通过 `indicatif` 进度条显示进度。

并发度：使用 `tokio` 任务配合有界信号量（默认 8 个并发 git 操作）。

### `east list`

1. 发现 workspace 根目录，resolve manifest。
2. 打印项目表格：name、path、revision、groups、是否已克隆。

### `east status`

1. 发现 workspace 根目录，resolve manifest。
2. 对每个已克隆项目检查：
   - 当前 HEAD 与期望 revision 是否一致。
   - 工作树是否 dirty/clean。
3. 打印带状态指示的表格。

### `east manifest --resolve`

1. 发现 workspace 根目录。
2. 解析完整 manifest（包括所有传递性 imports）。
3. 将解析后的 manifest 以 YAML 格式打印到 stdout。

## 7. 依赖（Phase 1）

| Crate | 依赖 | 用途 |
|---|---|---|
| `east-manifest` | `serde`、`serde_yaml` | YAML 解析 |
| `east-manifest` | `thiserror` | 错误类型 |
| `east-manifest` | `glob-match` | allowlist 模式匹配 |
| `east-vcs` | `tokio`（process） | 异步 git shell-out |
| `east-vcs` | `thiserror` | 错误类型 |
| `east-workspace` | `thiserror` | 错误类型 |
| `east-cli` | `clap`（derive） | CLI 参数解析 |
| `east-cli` | `anyhow`、`miette` | 错误诊断 |
| `east-cli` | `tokio`（full） | 异步运行时 |
| `east-cli` | `tracing`、`tracing-subscriber` | 日志 |
| `east-cli` | `indicatif` | 进度条 |

## 8. 测试策略

- **单元测试**（各 crate 源码文件中）：数据模型构造、serde 往返测试、group 过滤、URL 构造。
- **集成测试**（各 crate 的 `tests/` 目录）：从 YAML 字符串解析 manifest、使用 fixture 文件进行 import 解析、对 `tempfile` + `git init` 创建的临时仓库进行 git 操作。
- **CLI 集成测试**（顶层 `tests/`）：将 `east` 二进制作为子进程运行，对 fixture manifest 进行测试，验证退出码和输出。
- **Fixtures** 存放在 `tests/fixtures/`：代表 3–5 个项目 SDK 的最小 `east.yml` 文件。

## 9. 平台注意事项

- **Windows：** 测试 git 输出解析中的 CRLF 处理；全程使用 `std::path`（不硬编码 `/`）；对含空格路径加引号。
- **macOS：** `/var` 是 `/private/var` 的符号链接；比较前需 canonicalize 路径。
- **所有平台：** workspace 发现必须处理符号链接；import 解析的 visited 集合使用 `std::fs::canonicalize`。

## 10. Phase 1 非目标

- 配置系统（`east-config`）— 推迟到 Phase 2。
- 扩展命令（`east-command`）— 推迟到 Phase 2。
- Runner（`east-runner`）— 推迟到 Phase 3。
- CMake 集成（`east-build`）— 推迟到 Phase 3。
- `west.yml` 导入转换器 — 后续 Phase。
- `${workspace.root}` 以外的模板变量。
