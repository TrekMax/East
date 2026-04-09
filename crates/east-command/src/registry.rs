use std::collections::BTreeMap;
use std::path::PathBuf;

use east_manifest::{CommandDecl, Manifest};
use tracing::warn;

/// Where a command was discovered from.
#[derive(Debug, Clone)]
pub enum CommandSource {
    /// Declared in a manifest `commands:` section.
    Manifest,
    /// Found on `PATH` as an `east-<name>` executable.
    Path {
        /// Full path to the executable.
        executable: PathBuf,
    },
}

/// A resolved command ready for dispatch.
#[derive(Debug, Clone)]
pub struct ResolvedCommand {
    /// Command name.
    pub name: String,
    /// Short help text.
    pub help: String,
    /// Optional long help text.
    pub long_help: Option<String>,
    /// Where this command came from.
    pub source: CommandSource,
    /// The original declaration (only for manifest commands).
    pub decl: Option<CommandDecl>,
}

/// Registry of discovered extension commands.
///
/// Holds manifest-declared and PATH-discovered commands with collision
/// resolution: manifest commands always win over PATH commands.
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct CommandRegistry {
    commands: BTreeMap<String, ResolvedCommand>,
}

impl CommandRegistry {
    /// Build a registry from manifest-declared commands.
    #[must_use]
    pub fn from_manifest(manifest: &Manifest) -> Self {
        let mut commands = BTreeMap::new();
        for decl in &manifest.commands {
            let cmd = ResolvedCommand {
                name: decl.name.clone(),
                help: decl.help.clone(),
                long_help: decl.long_help.clone(),
                source: CommandSource::Manifest,
                decl: Some(decl.clone()),
            };
            commands.insert(decl.name.clone(), cmd);
        }
        Self { commands }
    }

    /// Discover `east-<name>` executables on `PATH` and add them.
    ///
    /// If a manifest command already exists with the same name, the PATH
    /// command is skipped and a warning is emitted.
    pub fn discover_path(&mut self, path_env: &str) {
        for dir in std::env::split_paths(&std::ffi::OsString::from(path_env)) {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();
                let cmd_name = extract_command_name(&name_str);
                let Some(cmd_name) = cmd_name else {
                    continue;
                };

                if self.commands.contains_key(&cmd_name) {
                    warn!("PATH command 'east-{cmd_name}' shadowed by manifest-declared command");
                    continue;
                }

                let cmd = ResolvedCommand {
                    name: cmd_name.clone(),
                    help: format!("(external command from PATH: {name_str})"),
                    long_help: None,
                    source: CommandSource::Path {
                        executable: entry.path(),
                    },
                    decl: None,
                };
                self.commands.insert(cmd_name, cmd);
            }
        }
    }

    /// Look up a command by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ResolvedCommand> {
        self.commands.get(name)
    }

    /// Iterate over all registered commands.
    pub fn iter(&self) -> impl Iterator<Item = &ResolvedCommand> {
        self.commands.values()
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Extract a command name from a filename like `east-foo` or `east-foo.exe`.
fn extract_command_name(filename: &str) -> Option<String> {
    let name = filename.strip_prefix("east-")?;
    // Strip known extensions
    let name = name
        .strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".cmd"))
        .or_else(|| name.strip_suffix(".bat"))
        .unwrap_or(name);
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}
