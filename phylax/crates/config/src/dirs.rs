use std::path::PathBuf;

use crate::PhConfig;

pub const DEFAULT_DIR_NAME: &str = "phylax";

/// The directory of the config in: `~/.config/phylax'
/// Refer to [dirs_next::cache_dir] for cross-platform behavior.

pub fn config_dir() -> PathBuf {
    let home = dirs_next::config_dir().expect("Can't find config dir");
    home.join(DEFAULT_DIR_NAME)
}

/// The path to the default config in: `~/.config/phylax/phylax.yaml'
/// Refer to [dirs_next::cache_dir] for cross-platform behavior.
pub fn config_path() -> PathBuf {
    let config_path = config_dir();
    config_path.join(PhConfig::DEFAULT_CONFIG_FILE)
}
/// Returns the path to the reth cache directory.
///
/// Refer to [dirs_next::cache_dir] for cross-platform behavior.
pub fn cache_dir() -> PathBuf {
    dirs_next::cache_dir().map(|root| root.join("phylax")).unwrap_or("./cache".into())
}

/// Returns the path to the reth logs directory.
///
/// Refer to [dirs_next::cache_dir] for cross-platform behavior.
pub fn logs_dir() -> PathBuf {
    cache_dir().join("logs")
}
