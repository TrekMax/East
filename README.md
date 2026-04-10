# east

A fast, manifest-driven development toolkit written in Rust.

[中文版](README.zh-CN.md)

## Overview

`east` is a general-purpose workspace management tool for any project that needs:

1. **Multi-repo management** — manifest-driven, with concurrent fetch
2. **Extension command mechanism** — define custom commands in your manifest
3. **Layered configuration system** — workspace, user, and project-level TOML config
4. **Pluggable runner abstraction** — extensible task execution framework

## Status

**Phase 2.6** — complete.

- **Phase 1:** Multi-repo management — `east init`, `east update`, `east list`, `east status`, `east manifest --resolve`
- **Phase 2:** Configuration & extension commands — `east config`, manifest-declared commands, PATH-based discovery, template engine
- **Phase 2.6:** Topology correction — manifest lives in a real git repo inside the workspace

## Quick Start

```bash
# Create a new workspace with a template manifest
east init

# Or use an existing local app as manifest repo
east init -l ./my-app

# Or clone a manifest repo from remote
east init -m https://github.com/your-org/sdk-manifest

# Update all projects declared in the manifest
east update

# Run a manifest-declared command
east hello

# Configure
east config set user.name trekmax
east config get user.name
```

## Installation

### From crates.io (recommended)

```bash
cargo install east-cli
```

### From Git

```bash
cargo install --git https://github.com/TrekMax/East east-cli
```

### From source

```bash
git clone https://github.com/TrekMax/East.git
cd East
cargo install --path crates/east-cli
```

### Build only (without installing)

```bash
cargo build --release
# Binary: target/release/east
```

The output is a single static binary with no Python runtime dependency.

## Supported Platforms

- Linux (x86_64, aarch64)
- macOS (universal)
- Windows (x86_64, aarch64)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
