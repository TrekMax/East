use std::path::PathBuf;

/// Provides config file paths for each layer.
///
/// Inject this trait to make config loading testable without touching
/// the real filesystem or home directory.
#[allow(clippy::module_name_repetitions)]
pub trait PathProvider {
    /// Path to the system-level config file, or `None` to skip.
    fn system_config_path(&self) -> Option<PathBuf>;
    /// Path to the global (user-level) config file, or `None` to skip.
    fn global_config_path(&self) -> Option<PathBuf>;
    /// Path to the workspace-level config file, or `None` to skip.
    fn workspace_config_path(&self) -> Option<PathBuf>;
}

/// Default path provider that reads platform-specific directories.
pub struct DefaultPathProvider {
    workspace_root: Option<PathBuf>,
}

impl DefaultPathProvider {
    /// Create a provider, optionally scoped to a workspace.
    #[must_use]
    pub const fn new(workspace_root: Option<PathBuf>) -> Self {
        Self { workspace_root }
    }
}

impl PathProvider for DefaultPathProvider {
    fn system_config_path(&self) -> Option<PathBuf> {
        system_config_dir().map(|d| d.join("config.toml"))
    }

    fn global_config_path(&self) -> Option<PathBuf> {
        global_config_dir().map(|d| d.join("config.toml"))
    }

    fn workspace_config_path(&self) -> Option<PathBuf> {
        self.workspace_root
            .as_ref()
            .map(|root| root.join(".east").join("config.toml"))
    }
}

/// Platform-specific system config directory.
#[allow(clippy::unnecessary_wraps)]
fn system_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("PROGRAMDATA").map(|p| PathBuf::from(p).join("east"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let path = PathBuf::from("/etc/east");
        Some(path)
    }
}

/// Platform-specific global (user) config directory.
fn global_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("east"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Respect XDG_CONFIG_HOME if set
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("east"));
        }
        // Fallback to ~/.config/east
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("east"))
    }
}
