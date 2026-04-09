# Phase 2 设计文档 — 配置系统与扩展命令

**状态：** 生效中
**范围：** `east config` CLI + 分层 TOML 配置；扩展命令发现、分发与模板引擎。

## 1. 目标

在 Phase 1 的多仓库核心之上交付两项正交能力：

1. **分层配置系统** — `east config` CLI、三层 TOML 合并（system / global / workspace）、library 层提供强类型访问。
2. **扩展命令机制** — 发现并分发用户自定义命令，来源为 `east.yml` 的 `commands:` 声明或 `PATH` 中的 `east-<name>` 可执行文件；附带变量替换模板引擎。

### 终态用户故事

- `east config set user.name trekmax` 持久化到 `~/.config/east/config.toml`。
- `east.yml` 中声明 `commands: [{ name: hello, exec: "echo hi from ${workspace.root} as ${config.user.name}" }]`，通过 `east hello` 调用。
- `PATH` 上的 `east-myext` 二进制通过 `east myext -- --some-flag` 调用。

### 显式非目标

- `east build`、`east flash`、`east debug`、`east attach`、`east reset`（Phase 3）。
- CMake 检测、OpenOCD、probe-rs、串口 runner。
- 动态库 / WASM 插件加载。
- 面向第三方 key 的 config schema 校验。
- `east config edit`（拉起 `$EDITOR`）。
- 动态发现命令的 tab 补全。

## 2. 配置系统

### 2.1 格式

TOML。不是 INI、YAML 或 JSON。

### 2.2 文件位置

| 层级 | Linux / macOS | Windows |
|---|---|---|
| System | `/etc/east/config.toml` | `%PROGRAMDATA%\east\config.toml` |
| Global | `$XDG_CONFIG_HOME/east/config.toml`（回退：`~/.config/east/config.toml`） | `%APPDATA%\east\config.toml` |
| Workspace | `<workspace_root>/.east/config.toml` | `<workspace_root>\.east\config.toml` |

### 2.3 合并语义

- **优先级（由低到高）：** system → global → workspace。
- 高优先级层按 key 覆盖低优先级层（嵌套表深度合并）。
- 缺失的层静默跳过（system 或 workspace 配置不存在不报错）。

### 2.4 Key 命名空间

点号路径：`user.name`、`update.parallelism`、`runner.default`。内部表示为嵌套 TOML table。

未知 key 允许且保留。`east` 核心保留：

- `user.name`、`user.email`
- `update.parallelism`（整数，默认 8）
- `runner.default`（字符串）
- `manifest.file`（字符串，默认 `east.yml`）

SDK 扩展可使用其他任意命名空间。

### 2.5 CLI 类型处理

`east config set KEY VALUE` 默认以字符串写入。类型标志：

- `--int` — 解析为整数
- `--bool` — 解析为布尔值（`true`/`false`）
- `--float` — 解析为浮点数

读取始终返回存储时的类型。

### 2.6 配置路径解析

配置文件路径通过可注入的 `PathProvider` trait 解析，确保测试时可不触碰真实文件系统。默认实现读取平台特定的环境变量和目录。

### 2.7 配置 I/O

- 同步操作。发生在 tokio 运行时启动之前或 `spawn_blocking` 中。
- 不使用 `tokio::fs` — 文件很小，异步 I/O 增加复杂度却无收益。
- 写入使用顶层点号 key 形式（`user.name = "x"`），避免多层合并歧义。

## 3. 扩展命令机制

### 3.1 发现来源（按顺序）

1. **Manifest 声明的命令**：来自解析后的 `east.yml` `commands:` 段。
2. **PATH 上的可执行文件**：匹配 `east-<name>`（Windows 同时检查 `PATHEXT` 扩展名）。

两个来源定义同名命令时，**manifest 声明优先**，并发出警告。

内建命令始终优先，不可被遮蔽：
`init`、`update`、`list`、`status`、`manifest`、`config`、`help`、`version`。

### 3.2 保留命令名

以下名称为未来内建命令保留，**不得**在 manifest 中声明：

`build`、`flash`、`debug`、`attach`、`reset`、`import-west`。

manifest 若声明这些名称，在加载时触发硬错误。

### 3.3 Manifest 声明命令的 Schema

```yaml
commands:
  - name: hello                       # 必需，[a-z][a-z0-9-]*
    help: "Say hello"                 # 必需，单行
    long-help: |                      # 可选，多行
      更详细的描述，由 `east help hello` 显示。
    exec: "echo hi from ${workspace.root}"   # exec | executable | script 三选一
    # executable: east-myext          # 委托给 PATH 上指定名称的二进制
    # script: scripts/hello.sh        # 相对于声明该命令的 manifest 文件的路径
    args:                             # 可选，声明式参数 schema
      - name: target
        help: "Target name"
        required: false
        default: "world"
    env:                              # 可选，额外环境变量
      FOO: "bar"
    cwd: "${workspace.root}"          # 可选，工作目录
```

**校验规则：**

- `exec`、`executable`、`script` 必须且只能出现其中一个。
- `name` 必须匹配 `[a-z][a-z0-9-]*`。
- `name` 不得与内建或保留命令名冲突。
- 违规是 manifest 校验错误，在加载 `east.yml` 时报出。

### 3.4 Shell 执行规则

对于模板渲染后的 `exec:` 命令：

- **Unix：** `sh -c <rendered_string>`
- **Windows：** `cmd /C <rendered_string>`

对于 `script:` 命令：

- 脚本路径**相对于声明该命令的 manifest 文件**解析，不是相对于 `cwd`。
- 脚本直接调用（Unix 上必须可执行，Windows 上需有适当扩展名）。

对于 `executable:` 命令：

- 按给定名称在 `PATH` 中查找。

### 3.5 参数传递

- **Manifest 声明的 args：** 填充 `${arg.name}`，在分发时由动态构建的 clap `Command` 解析。
- **`--` 之后的 token：** 原样透传给 exec/executable/script，追加在 manifest 声明 args 之后。
- **仅 PATH 上存在、无 manifest 声明的命令：** 子命令名之后的全部 token 原样透传。

## 4. 模板引擎

### 4.1 语法

仅 `${namespace.key}`。无 filter、条件或循环。

### 4.2 命名空间

| 模式 | 描述 |
|---|---|
| `${workspace.root}` | workspace 根目录绝对路径 |
| `${workspace.manifest}` | 顶层 `east.yml` 绝对路径 |
| `${project.<name>.path}` | 项目检出目录绝对路径 |
| `${project.<name>.revision}` | 解析后的 revision 字符串 |
| `${config.<dotted.key>}` | 合并后配置的值，转为字符串 |
| `${env.<NAME>}` | 环境变量 |
| `${arg.<name>}` | 当前命令的声明参数值 |

### 4.3 Key 缺失行为

硬错误。不做静默空串替换。错误信息必须标识模板来源（manifest 文件路径，尽可能带行号）。

### 4.4 转义

`$${...}` 产出字面量 `${...}`。无其他特殊字符。

### 4.5 实现

手写约 80 行。不引入模板引擎 crate。

## 5. Crate 依赖图

```
east-cli  ─────────────┬─► east-command ─► east-manifest
                       │                │
                       ├─► east-config ◄┘
                       ├─► east-workspace
                       └─► east-vcs
```

规则：

- `east-config` 不依赖 `east-manifest`。
- `east-command` 同时依赖 `east-config` 与 `east-manifest`。
- 两者都不依赖 `east-cli`。

## 6. 错误模型

| Crate | 错误类型 | 变体 |
|---|---|---|
| `east-config` | `ConfigError` | `Io`、`TomlParse`、`TomlSerialize`、`KeyNotFound`、`TypeMismatch` |
| `east-command` | `CommandError` | `InvalidName`、`MutuallyExclusiveFields`、`ReservedName`、`TemplateError`、`SpawnFailed`、`NotFound` |

模板错误是 `east-command` 内的子枚举：

| `TemplateError` 变体 | 含义 |
|---|---|
| `UnknownNamespace` | 命名空间前缀无法识别 |
| `MissingKey` | 给定命名空间中找不到 key |
| `UnterminatedVariable` | `${` 无匹配的 `}` |
| `InvalidSyntax` | 其他解析失败 |

## 7. 性能约束

- `east --version` 必须保持在 30 ms 以内。配置与命令发现惰性加载。
- 配置加载静默跳过缺失的层。
- async 运行时中禁止阻塞 I/O。

## 8. 测试策略

- **配置目录：** 通过 `PathProvider` trait 注入；测试中绝不读取真实 `$HOME`。
- **PATH 发现：** 在临时目录创建假 `east-foo` 可执行文件；仅为子进程前置到 `PATH`。
- **环境变量隔离：** 使用 `assert_cmd`（子进程）或全局 mutex 处理环境变量相关测试。
- **Windows 覆盖：** 每套新测试必须至少包含一条 Windows 特有断言。
- **Fixtures：** 命令相关的新 manifest 放在 `tests/fixtures/phase2/`。
