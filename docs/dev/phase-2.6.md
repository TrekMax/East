# Phase 2.6 Development Notes

## What Was Delivered

Workspace topology corrected: manifest now lives in a real git repo, sibling of `.east/`.

### New `east init` Modes

- **Mode L** (`-l <path>`): use an existing local directory as manifest repo
- **Mode M** (`-m <url>`): clone a remote repository as manifest repo
- **Mode T** (default): create a template manifest repo with `git init`

### Infrastructure Changes

- `ManifestSelf` struct: optional `self:` section in `east.yml` with `path` hint
- `ManifestConfig` in `east-config`: `[manifest]` section with `path` and `file` fields, validation
- `Workspace` rewrite: loads config first, derives manifest path from `[manifest]` section
- `manifest_repo_path()` and `manifest_file_path()` APIs on `Workspace`
- Legacy fallback: workspaces without `[manifest]` config fall back to `root/east.yml`

### Breaking Change

Existing workspaces must be re-initialized. Old `east init <url>` positional syntax removed, replaced by `east init -m <url>`.

## Test Summary

- 5 manifest self: tests
- 7 config [manifest] tests
- 4 workspace topology tests
- 8 init mode tests (L, T, end-to-end)
- 10 update tests (migrated to new topology)
- **Total: 165 tests**, all passing

## What Went Well

1. **Legacy fallback was key.** Commands that use `ws.manifest_path()` work with both old and new topology because of the fallback logic. This made migration incremental.

2. **`ManifestConfig` validation is clean.** Rejects absolute, empty, and `..`-containing paths at both read and write time.

3. **Test migration was mechanical.** The update tests only needed their setup helper changed (use `east init -l` + `east update` instead of old `east init <url>`).

## What Was Hard

1. **Stale build artifacts.** After renaming test files, cargo used cached old binaries. Required `cargo clean` to fix. CI won't have this problem.

2. **`do_update()` hardcoded `east.yml` join.** Needed to change it to discover the workspace and use `manifest_path()` instead.

## Decisions Made

1. **No auto-update after init.** `east init` in all three modes does NOT automatically run `east update`. This matches west's behavior — init and update are separate steps.

2. **Legacy config fallback.** Rather than hard-erroring when `[manifest]` is missing, the workspace falls back to `root/east.yml`. This eases migration for tests and existing workflows.
