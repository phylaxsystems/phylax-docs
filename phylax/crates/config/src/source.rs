use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{dirs::config_path, PhConfig};

/// Configuration source structure
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSource {
    pub path: PathBuf, // Path of the configuration
}

impl Default for ConfigSource {
    /// Default configuration source
    fn default() -> Self {
        Self { path: config_path().join(PhConfig::DEFAULT_CONFIG_FILE) }
    }
}

impl Display for ConfigSource {
    /// Display the configuration source
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "path:{}", self.path.display())
    }
}
