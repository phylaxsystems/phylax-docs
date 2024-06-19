use crate::{substitute_env::deserialize_with_env, Trigger};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AlertConfig {
    /// Name of the Alert
    #[serde(deserialize_with = "deserialize_with_env")]
    pub name: String,
    /// Triggers of the Alert
    pub on: Vec<Trigger>,
    #[serde(flatten)]
    pub kind: AlertKind,
    /// The minimum amount of time to wait between executions of the action
    /// (in seconds)
    #[serde(default)]
    pub delay: u32,
}

/// The type of the alert.
/// - An EVM alert (uses Foundry)
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum AlertKind {
    Evm(EvmAlertConfig),
}

/// The configuration for an alert
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct EvmAlertConfig {
    #[serde(default = "EvmAlertConfig::default_project_root")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub foundry_project_root_path: PathBuf,
    #[serde(default = "EvmAlertConfig::default_profile")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub foundry_profile: String,
}

impl EvmAlertConfig {
    /// Defaults to `./alerts`, in relation to the root of the Phylax config
    pub fn default_project_root() -> PathBuf {
        "./alerts".into()
    }
    pub fn default_profile() -> String {
        "default".to_owned()
    }
}

#[allow(irrefutable_let_patterns)]
impl AlertConfig {
    /// Update the paths in the AlertConfig to be canonicalized
    pub fn with_canonicalized_paths(&mut self, root: PathBuf) -> Result<()> {
        match &mut self.kind {
            AlertKind::Evm(conf) => {
                let new_path = root.join(&conf.foundry_project_root_path);
                if !new_path.exists() {
                    return Err(eyre::eyre!("Path does not exist: {:?}", new_path));
                }
                conf.foundry_project_root_path = new_path;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::try_absolute_path_of;
    use std::path::Path;

    #[test]
    fn test_default_project_root() {
        let default_path = EvmAlertConfig::default_project_root();
        assert_eq!(default_path, Path::new("./alerts"));
    }

    #[test]
    fn test_with_canonicalized_paths() {
        let temp_dir = tempfile::TempDir::new().expect("unable to create temporary directory");
        let evm = EvmAlertConfig {
            foundry_project_root_path: temp_dir.path().to_path_buf(),
            foundry_profile: "default".to_string(),
        };
        let mut alert_config = AlertConfig {
            name: "Test Alert".to_string(),
            on: vec![],
            kind: AlertKind::Evm(evm),
            delay: 0,
        };
        let root_path = try_absolute_path_of(Path::new(".")).expect("unable to get absolute path");
        let result = alert_config.with_canonicalized_paths(root_path.clone());
        assert!(result.is_ok());
        match alert_config.kind {
            AlertKind::Evm(conf) => {
                assert_eq!(conf.foundry_project_root_path, root_path.join(temp_dir.path()));
            }
        }
    }
    #[test]
    fn test_with_nonexistent_paths() {
        let evm = EvmAlertConfig {
            foundry_project_root_path: "./nonexistent".into(),
            foundry_profile: "default".to_string(),
        };
        let mut alert_config = AlertConfig {
            name: "Test Alert".to_string(),
            on: vec![],
            kind: AlertKind::Evm(evm),
            delay: 0,
        };
        let root_path = try_absolute_path_of(Path::new(".")).unwrap();
        let result = alert_config.with_canonicalized_paths(root_path.clone());
        assert!(result.is_err());
    }
}
