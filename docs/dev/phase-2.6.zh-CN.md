# Phase 2.6 开发记录

## 交付内容

修正 workspace 拓扑：manifest 现在存放于真实 git 仓库中，与 `.east/` 平级。

### 新的 `east init` 模式

- **Mode L**（`-l <path>`）：使用已有本地目录作为 manifest 仓库
- **Mode M**（`-m <url>`）：从远端克隆仓库作为 manifest 仓库
- **Mode T**（默认）：创建模板 manifest 仓库并 `git init`

### 基础设施变化

- `ManifestSelf` 结构体：`east.yml` 中可选的 `self:` 段，带 `path` 提示
- `east-config` 中的 `ManifestConfig`：`[manifest]` 段，含 `path` 和 `file` 字段及校验
- `Workspace` 重写：先加载 config，从 `[manifest]` 段推导 manifest 路径
- `Workspace` 上新增 `manifest_repo_path()` 和 `manifest_file_path()` API
- 旧版兼容回退：缺少 `[manifest]` 配置的 workspace 回退到 `root/east.yml`

### 破坏性变更

已有 workspace 必须重新初始化。旧的 `east init <url>` 位置参数语法已移除，替换为 `east init -m <url>`。

## 测试概览

- 5 个 manifest self: 测试
- 7 个 config [manifest] 测试
- 4 个 workspace 拓扑测试
- 8 个 init 模式测试（L、T、端到端）
- 10 个 update 测试（已迁移到新拓扑）
- **总计：165 个测试**，全部通过

## 顺利的部分

1. **旧版回退是关键。** 使用 `ws.manifest_path()` 的命令在新旧拓扑下都能工作，因为有回退逻辑。这使迁移可以增量进行。

2. **`ManifestConfig` 校验很干净。** 在读取和写入时都拒绝绝对、空和含 `..` 的路径。

3. **测试迁移是机械性的。** update 测试只需修改 setup 辅助函数（用 `east init -l` + `east update` 替代旧的 `east init <url>`）。

## 困难的部分

1. **过期的构建缓存。** 重命名测试文件后，cargo 使用了旧的缓存二进制。需要 `cargo clean` 修复。CI 不会有此问题。

2. **`do_update()` 硬编码了 `east.yml` 拼接。** 需要改为通过 workspace 发现并使用 `manifest_path()`。

## 做出的决策

1. **init 后不自动 update。** 三种 init 模式都不自动运行 `east update`。这与 west 行为一致——init 和 update 是独立步骤。

2. **旧版 config 回退。** 当 `[manifest]` 缺失时不硬错误，而是回退到 `root/east.yml`。这缓解了测试和现有工作流的迁移压力。
