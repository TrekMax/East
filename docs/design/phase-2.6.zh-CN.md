# Phase 2.6 设计文档 — 拓扑修正

**状态：** 生效中
**范围：** 修正 workspace 拓扑，让 manifest 存放于真实 git 仓库中，而非 workspace 根的裸文件。对已有 workspace 是破坏性变更。

## 1. 本 Phase 存在的原因

Phase 1 选择从 manifest 仓库提取 `east.yml` 后丢弃克隆。这是一个建模错误，代价包括：

- manifest 无历史（裸文件，无 git 上下文）。
- 无法从上游更新 manifest。
- manifest 变更没有 PR 工作流。
- 本地与上游 manifest 静默分叉风险。
- **自然和 `east.yml` 共存的文件**（OpenOCD 配置、toolchain 文件、构建脚本、T3 拓扑中的应用源码）在 manifest 被提取为裸文件时丢失。

Phase 2.6 采用类似 west T1/T2/T3 拓扑的模型：manifest 仓库始终是 workspace 内的真实 git repo，与 `.east/` 平级。

作者的主力场景是 **T3（Application）**：manifest repo 就是应用本身，`east.yml` 声明依赖，`east build` 直接构建应用。

## 2. Workspace 布局

```
<workspace-root>/
├── .east/
│   ├── config.toml      # 包含 [manifest] 段
│   └── state.toml
├── <manifest-repo>/     # 真实 git repo，与 .east/ 平级
│   ├── .git/
│   ├── east.yml
│   └──（OpenOCD cfg、src/、CMakeLists.txt 等）
├── <project-a>/         # 由 east update 获取
├── <project-b>/
└── ...
```

关键性质：

- `.east/` 标记 workspace 根（发现逻辑与 Phase 1 相同）。
- manifest repo 是**真实的 git repo**，绝不是裸目录。
- manifest repo 是 `.east/` 的**兄弟**，不是子目录。

## 3. `east init` — 三种模式

### Mode L — 本地已有 repo

```
east init -l [--mf FILE] <local-path>
```

- `<local-path>` 必须存在且包含 manifest 文件（默认 `east.yml`）。
- `.east/` 创建在 `<local-path>` 的**父目录**。
- 不自动运行 `east update`。

### Mode M — 从远端克隆

```
east init -m <url> [--mr REV] [--mf FILE] [<workspace-dir>]
```

- 将 `<url>` clone 到 `<workspace-dir>/<repo-name>/`。
- `<repo-name>` 从 URL basename 去掉 `.git` 得出，或使用克隆后 manifest 中 `self.path` 的值。
- 若给了 `--mr`，checkout 该 revision。
- `.east/` 创建在 `<workspace-dir>`。
- 不自动运行 `east update`。

### Mode T — 模板（默认）

```
east init [<dir>]
```

- `<dir>` 默认为 `manifest`。
- 创建模板 `east.yml`、`.gitignore`，运行 `git init`。
- 不添加 remote，不做 initial commit。
- `.east/` 创建在 CWD。

三种模式中：`.east/` 已存在 = 硬错误，除非 `--force`。

## 4. `config.toml` — `[manifest]` 段

```toml
[manifest]
path = "my-app"        # workspace 相对路径，指向 manifest repo
file = "east.yml"      # manifest 文件名，相对于 manifest.path
```

- 由 `east init` 写入，被 `Workspace::load()` 读取。
- 仅限 workspace 配置层。
- 校验：`path` 必须相对、非空、无 `..`、非绝对。TOML 中只用正斜杠。

## 5. Manifest `self:` 段（可选）

```yaml
version: 1
self:
  path: my-app          # 期望的 workspace 路径提示
```

- 完全可选。不带 `self:` 的 manifest 照常工作。
- Mode L：与 init 参数 basename 不匹配 = 警告（非错误）。
- Mode M：若存在，覆盖 URL 派生的 repo-name 作为 clone 目录名。
- Mode T：模板中以注释形式包含作为文档。
- 未来保留字段：`description`、`maintainers`、`repo-url` — 解析并忽略。

## 6. Workspace API 变化

`Workspace` 新方法：

```rust
pub fn manifest_repo_path(&self) -> &Path;
pub fn manifest_file_path(&self) -> &Path;
```

新加载顺序：

1. 发现 `.east/`（从 CWD 向上查找）。
2. 从 `.east/config.toml` 加载 config，提取 `[manifest]`。
3. 计算 `manifest_repo_path` 和 `manifest_file_path`。
4. 从 `manifest_file_path` 加载 manifest。
5. 从 `.east/state.toml` 加载 state。

Phase 1/2 不兼容的错误消息必须清晰可行动。

## 7. `east update` 行为

- 不 fetch/checkout manifest repo 自身。
- 读取当前 checkout 的 manifest（尊重未 commit 的本地修改）。
- 用户通过普通 git 管理 manifest repo。

## 8. 错误模型

| 错误 | 描述 |
|---|---|
| `ConfigError::ManifestSectionMissing` | 检测到 Phase 1/2 workspace，给出升级提示 |
| `ConfigError::InvalidManifestPath` | 绝对、空或含 `..` |
| `WorkspaceError::ManifestFileNotFound` | manifest 文件在计算路径处缺失 |
| `WorkspaceError::AlreadyInitialized` | `.east/` 已存在且未给 `--force` |

`ManifestError::SelfPathMismatch` 是通过 `tracing::warn!` 发出的**警告**，不是硬错误。

## 9. 非目标

- 无 Phase 3 功能（build、runner、state.toml schema 变化）。
- 不自动更新 manifest repo。
- 无 Phase 1/2 workspace 迁移工具。
- 不支持 submodules、multi-manifest。
- 不跟踪 `manifest.revision`。
