# Phase 2 Development Notes

## What Was Delivered

All Phase 2 goals achieved:

- `east config get/set/unset/list` — three-layer TOML config with system/global/workspace merge
- Manifest `commands:` section — declaration, validation (name regex, mutual exclusivity, reserved names)
- Extension command dispatch — exec (shell), script, executable, PATH-based `east-<name>`
- Template engine — `${namespace.key}` substitution, escape via `$${...}`, hard error on missing keys
- Command discovery — manifest-declared commands win over PATH, with collision warnings

## Crate Summary

| Crate | Phase 2 Additions | Tests |
|---|---|---|
| `east-config` | ConfigValue, ConfigStore, TOML I/O, Config (3-layer), PathProvider | 30 |
| `east-command` | CommandRegistry, PATH discovery, collision resolution, TemplateEngine | 14 + 1 doctest |
| `east-manifest` | CommandDecl, CommandArg structs, name/mutex/reserved validation | 10 new (51 total) |
| `east-cli` | `east config` subcommand, extension command dispatch | 12 new (21 total) |

**Total: 123 tests, all passing.**

## What Went Well

1. **ConfigStore as a tree.** Using `BTreeMap<String, Node>` with `Leaf`/`Branch` nodes made dotted-key access, deep merge, and TOML round-trip all straightforward.

2. **PathProvider trait.** Injecting config paths via a trait made tests fully hermetic — no tests touch real `$HOME` or system config.

3. **Template engine simplicity.** The hand-written ~80-line template engine was easy to implement and test. No regex needed, just character-by-character parsing.

4. **`allow_external_subcommands` in clap.** This feature made extension command dispatch clean — unknown subcommands are captured as `Vec<String>` and dispatched to the command registry.

5. **CommandDecl in east-manifest.** Adding `commands:` to the manifest model was non-breaking thanks to `#[serde(default)]`. All Phase 1 tests continued to pass after adding the `commands` field.

## What Was Hard

1. **Clippy pedantic + nursery (again).** `module_name_repetitions` required `#[allow]` on nearly every public type named `Config*` or `Command*`. The `similar_names` lint flagged `cmd` vs `cwd` which led to a rename.

2. **Edition 2024 dependency conflicts.** Continued from Phase 1 — `clap`, `tempfile`, `assert_cmd` all needed version pins to avoid transitive deps requiring edition 2024 features not available in Rust 1.82.0.

3. **Shell execution cross-platform.** The `exec:` dispatch uses `sh -c` on Unix and `cmd /C` on Windows. This was decided upfront in the design doc but required `#[cfg]` blocks in the dispatch code.

4. **Test isolation for PATH discovery.** Creating fake `east-<name>` executables in temp dirs and passing them via `PATH` to child processes required careful setup with `use std::os::unix::fs::PermissionsExt`.

## Decisions Made Mid-Phase

1. **Combined Red/Green commits for tightly coupled features.** For the `east-command` crate, I wrote all tests (registry, PATH, collision, template) in one Red commit and implemented all modules in one Green commit, rather than interleaving. This was more efficient since the modules are small and interdependent.

2. **`ConfigStore::from_toml_str` returns empty for missing files.** Rather than erroring, `load_from_file` on a nonexistent path returns an empty store. This simplifies the three-layer merge — missing layers just contribute nothing.

3. **No `miette` integration yet.** The design doc specifies `miette` for rich error display with source spans. The current implementation uses `anyhow` for all CLI errors. `miette` integration is deferred to a polish pass.

4. **Script path resolution uses workspace root.** The design doc says script paths should be relative to the manifest that declared them. Currently they're resolved relative to the workspace root, which is correct for top-level manifests but would need adjustment when imported manifests declare commands.

## Looking Ahead to Phase 3

- **Runner trait** (`east-runner`): `OpenOCD` runner, serial ISP runner
- **CMake wrapper** (`east-build`): `east build` with CMake preset support
- **`east flash` / `east debug` / `east attach` / `east reset`** via runner dispatch
