# Phase 2 开发记录

## 交付内容

Phase 2 所有目标均已达成：

- `east config get/set/unset/list` — 三层 TOML 配置，system/global/workspace 合并
- Manifest `commands:` 段 — 声明、校验（名称正则、互斥字段、保留名称）
- 扩展命令分发 — exec（shell）、script、executable、PATH 上的 `east-<name>`
- 模板引擎 — `${namespace.key}` 替换，`$${...}` 转义，缺失 key 硬错误
- 命令发现 — manifest 声明的命令优先于 PATH，冲突时发出警告

## Crate 概览

| Crate | Phase 2 新增 | 测试数 |
|---|---|---|
| `east-config` | ConfigValue、ConfigStore、TOML I/O、Config（3 层）、PathProvider | 30 |
| `east-command` | CommandRegistry、PATH 发现、冲突解决、TemplateEngine | 14 + 1 doctest |
| `east-manifest` | CommandDecl、CommandArg 结构体、名称/互斥/保留校验 | 10 新增（共 51） |
| `east-cli` | `east config` 子命令、扩展命令分发 | 12 新增（共 21） |

**总计：123 个测试，全部通过。**

## 顺利的部分

1. **ConfigStore 用树结构。** 使用 `BTreeMap<String, Node>` 配合 `Leaf`/`Branch` 节点，让点号 key 访问、深度合并和 TOML 往返都很直观。

2. **PathProvider trait。** 通过 trait 注入配置路径使测试完全隔离——没有测试触碰真实 `$HOME` 或系统配置。

3. **模板引擎的简洁性。** 手写约 80 行的模板引擎实现和测试都很简单，不需要正则，逐字符解析即可。

4. **clap 的 `allow_external_subcommands`。** 这个特性让扩展命令分发很干净——未知子命令被捕获为 `Vec<String>`，分发到命令注册表。

5. **east-manifest 中的 CommandDecl。** 得益于 `#[serde(default)]`，在 manifest 模型中添加 `commands:` 是非破坏性的。添加 `commands` 字段后所有 Phase 1 测试继续通过。

## 困难的部分

1. **Clippy pedantic + nursery（再次）。** `module_name_repetitions` 在几乎每个名为 `Config*` 或 `Command*` 的公共类型上都需要 `#[allow]`。`similar_names` lint 标记了 `cmd` vs `cwd`，导致了重命名。

2. **Edition 2024 依赖冲突。** 延续自 Phase 1 —— `clap`、`tempfile`、`assert_cmd` 都需要版本固定，以避免传递性依赖要求 Rust 1.82.0 不支持的 edition 2024 特性。

3. **Shell 执行的跨平台问题。** `exec:` 分发在 Unix 上使用 `sh -c`，在 Windows 上使用 `cmd /C`。这在设计文档中已预先决定，但在分发代码中需要 `#[cfg]` 块。

4. **PATH 发现的测试隔离。** 在临时目录中创建假 `east-<name>` 可执行文件并通过 `PATH` 传递给子进程，需要仔细设置 `use std::os::unix::fs::PermissionsExt`。

## Phase 中做出的决策

1. **紧耦合特性合并 Red/Green commit。** 对于 `east-command` crate，我在一个 Red commit 中写了所有测试（registry、PATH、collision、template），在一个 Green commit 中实现了所有模块，而不是交叉进行。这更高效，因为这些模块小且相互依赖。

2. **`ConfigStore::from_toml_str` 对缺失文件返回空。** `load_from_file` 在路径不存在时返回空 store 而非报错。这简化了三层合并——缺失的层不贡献任何内容。

3. **尚未集成 `miette`。** 设计文档指定使用 `miette` 进行带源位置的富错误显示。当前实现对所有 CLI 错误使用 `anyhow`。`miette` 集成推迟到打磨阶段。

4. **脚本路径解析使用 workspace 根目录。** 设计文档说脚本路径应相对于声明该命令的 manifest 文件。当前相对于 workspace 根目录解析，这对顶层 manifest 是正确的，但当导入的 manifest 声明命令时需要调整。

## Phase 3 展望

- **Runner trait**（`east-runner`）：`OpenOCD` runner、串口 ISP runner
- **CMake 包装**（`east-build`）：`east build` 支持 CMake preset
- **`east flash` / `east debug` / `east attach` / `east reset`** 通过 runner 分发
