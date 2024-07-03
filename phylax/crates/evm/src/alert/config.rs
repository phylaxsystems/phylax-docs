use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents the configuration for an EVM-based alert system.
/// This configuration primarily includes the path to the alert configuration file.
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvmAlertConfig {
    /// The file path where the alert configuration is stored.
    #[serde(default)]
    pub alert_path: PathBuf,
}
