#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc(issue_tracker_base_url = "https://github.com/phylax-systems/phylax/issues/")]

//! PhConfig is the central configuration for Phylax, which defines all it's behaviour.
//
//! The configuration is a collection of smaller configurations, that each configures a
//! particular part of Phylax.
//
//! The configuration is expected to be provided in the form of a YAML file:
//!```yaml
//!    tracing:
//!      logLevel: trace
//!      logsDir: testdata/logs
//!    api:
//!      bindIp: 127.0.0.1:4269
//!    alerts:
//!      - name: protocol-alert
//!        on:
//!          - rule: 'type == "new_block"'
//!            interval: 1
//!        foundryProjectRootPath: ./alerts
//!    watchers:
//!      - name: sepolia
//!        chain: sepolia
//!        ethRpc: "$PH_RPC_SEPOLIA"
//!    actions:
//!      - name: kill-protocol
//!        on:
//!          - rule: "type == \"alert_execution\" && alert_state == \"on\""
//!            interval: 1
//!        contractName: KillProtocol
//!        contractFilename: KillProtocol.s.sol
//!        wallet: $WALLET
//!        foundryProjectRootPath: ./actions/kill-protocol
//!      - name: slack-webhook
//!        on:
//!          - rule: "type == \"alert_execution\" && alert_state == \"on\""
//!            interval: 1
//!        target: $SLACK_WEBHOOK
//!        platform: slack
//!        title: "Validator Collateral check"
//!        description: "This is a test"
//! ```
//!
//! A few notes:
//! - `alerts`, `actions`, `watchers` are lists
//! - You can read more about the respective configs by openning the type in the docs
//! - The config supports env variable substitution. All `$ENV_VARIABLE` will be replaced
//! with the env variable `ENV_VARIABLE`. If the env variable is not found, it will produce an
//! error.
//! - Not all fields support the env variable substitution. Read the docs of every sub-config to see
//!   which support.
//! Import necessary modules and libraries
use config::{builder, Config, ConfigBuilder, File, ValueKind};
use eyre::{Result, WrapErr};
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
// Re-export configs
pub use crate::{
    action::{
        ActionConfig, ActionKind, ChatPlatform, ChatWebhookConfig, EvmActionConfig,
        GeneralWebhookConfig, WalletKind, WebhookAuth,
    },
    alert::{AlertConfig, AlertKind, EvmAlertConfig},
    api::ApiConfig,
    dirs::config_dir,
    metrics::MetricsConfig,
    orchestrator::OrchestratorConfig,
    sensitive::SensitiveValue,
    source::ConfigSource,
    tracing_config::{LogLevel, TracingConfig},
    watcher::{EvmWatcherConfig, WatcherConfig, WatcherKind},
};
use phylax_tracing::tracing::{self, instrument};

// Declare public modules
mod action;
mod alert;
mod api;
mod dirs;
mod error;
mod metrics;
mod orchestrator;
mod sensitive;
mod source;
mod substitute_env;
mod tracing_config;
mod watcher;

/// Prefix for environment variables
pub const ENV_VAR_PREFIX: &str = "PH";
pub const METRICS_TASK_NAME: &str = "metrics";
pub const API_TASK_NAME: &str = "api";
pub const ORCHESTRATOR_TASK_NAME: &str = "orchestrator";
/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct PhConfig {
    #[serde(skip_serializing)]
    /// Source of the configuration
    pub source: PathBuf,
    /// Flag to watch the configuration
    pub watch_config: bool,
    /// The config of the tracing instrumentation
    pub tracing: TracingConfig,
    /// The config of the API
    pub api: ApiConfig,
    /// The configs of the active alerts
    pub alerts: Vec<AlertConfig>,
    /// The configs of the active watchers
    pub watchers: Vec<WatcherConfig>,
    /// The configs of the active actions
    pub actions: Vec<ActionConfig>,
    #[serde(skip_deserializing, skip_serializing)]
    /// The config for the Orchestrator
    pub orchestrator: OrchestratorConfig,
    /// The config for the Metrics task
    #[serde(skip_deserializing, skip_serializing)]
    pub metrics: MetricsConfig,
}

/// Configuration builder structure
#[derive(Debug)]
pub struct PhConfigBuilder {
    pub builder: ConfigBuilder<builder::DefaultState>, // Configuration builder
}

impl PhConfigBuilder {
    /// Create a new default config builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file as a source
    pub fn with_source_from_path(mut self, path: PathBuf) -> Self {
        self.builder =
            self.builder.add_source(File::new(&path.to_string_lossy(), config::FileFormat::Yaml));
        self
    }

    /// Add a config instance as a source
    pub fn with_source_from_config<T: Into<PhConfig> + Serialize>(mut self, config: T) -> Self {
        self.builder = self
            .builder
            .add_source(config::Config::try_from(&config).expect("failed to convert to Config"));
        self
    }

    /// Add a default source to the config builder
    ///
    /// This function will create a default `PhConfig` and add it as a source to the config builder.
    pub fn with_source_from_default(self) -> Self {
        // Create default config
        let default = PhConfig::default();
        self.with_source_from_config(default)
    }

    /// Try to build a config `PhConfig` from a `PhConfigBuilder` and then canonicalize
    /// paths
    #[instrument(err, skip(self))]
    pub fn try_build(self) -> Result<PhConfig> {
        let config = self
            .builder
            .build()
            .wrap_err("Config Builder failed to build the config from the sources")?;
        let mut final_config: PhConfig = serde_path_to_error::deserialize(config)
            .map_err(|err| eyre::Report::msg(err.to_string()))?;
        // canonicalize_paths
        final_config = final_config.with_canonicalized_paths()?;
        final_config.check_duplicate_task_names()?;
        final_config.set_orchestrator_triggers();
        Ok(final_config)
    }

    /// Set an override for a key-value pair
    pub fn set_override<T: Into<ValueKind>>(mut self, key: &str, value: T) -> Result<Self> {
        self.builder = self.builder.set_override(key, value)?;
        Ok(self)
    }

    /// Set an override for a key-value pair with an option
    pub fn set_override_option<T: Into<ValueKind>>(
        mut self,
        key: &str,
        value: Option<T>,
    ) -> Result<Self> {
        self.builder = self.builder.set_override_option(key, value)?;
        Ok(self)
    }
}

impl PhConfig {
    pub const DEFAULT_CONFIG_FILE: &'static str = "phylax.yaml";

    /// Set the orchestrator triggers based on the current configuration
    pub fn set_orchestrator_triggers(&mut self) {
        self.orchestrator.on = OrchestratorConfig::api_or_metrics_triggers(self);
    }

    /// This function takes the current instance of `PhConfig` and modifies its paths to be
    /// absolute. It first converts the `source.path` and `tracing.logs_dir` to absolute paths.
    /// Then, it gets the current directory and the parent of `source.path` as the root directory.
    /// If the parent of `source.path` is not available, it uses the current directory as the root.
    /// After that, it iterates over the `alerts` and `actions` and converts their paths to absolute
    /// paths.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The conversion of `source.path` or `tracing.logs_dir` to absolute paths fails.
    /// - The conversion of paths in `alerts` or `actions` to absolute paths fails.
    ///
    /// # Returns
    ///
    /// This function returns the current instance of `PhConfig` with all paths converted to
    /// absolute paths.
    pub fn with_canonicalized_paths(mut self) -> Result<Self> {
        self.source = try_absolute_path_of(&self.source)?;
        self.tracing.logs_dir = try_absolute_path_of(&self.tracing.logs_dir)?;
        let current = std::env::current_dir()?;
        let root = self.source.parent().unwrap_or(&current);
        self.alerts = self
            .alerts
            .into_iter()
            .map(|mut alert| -> Result<AlertConfig> {
                {
                    alert.with_canonicalized_paths(root.into())?;
                    Ok(alert)
                }
            })
            .collect::<Result<Vec<AlertConfig>>>()?;
        self.actions = self
            .actions
            .into_iter()
            .map(|mut action| -> Result<ActionConfig> {
                {
                    action.with_canonicalized_paths(root)?;
                    Ok(action)
                }
            })
            .collect::<Result<Vec<ActionConfig>>>()?;
        Ok(self)
    }

    /// Checks for duplicated task names across system tasks and custom tasks (actions, watchers,
    /// and alerts).
    ///
    /// System tasks are predefined tasks that have reserved names.
    /// Custom tasks are user-defined or loaded tasks that can be actions, watchers, or alerts.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - A task name is duplicated across any of the system tasks or custom tasks.
    /// - A task name of a custom task coincides with any of the system tasks' names.
    ///
    /// The error will contain a message indicating the duplicated task name and a reminder that
    /// task names must be unique.
    pub fn check_duplicate_task_names(&mut self) -> Result<()> {
        let mut task_names = HashSet::new();
        let system_tasks = vec![API_TASK_NAME, ORCHESTRATOR_TASK_NAME, METRICS_TASK_NAME];
        for task in system_tasks {
            task_names.insert(task.to_string());
        }
        for task in &mut self.actions {
            if task_names.contains(&task.name) {
                return Err(eyre::eyre!(
                    "Task name '{}' is duplicated. Task names must be unique",
                    task.name
                ));
            }
            task_names.insert(task.name.clone());
        }
        for task in &mut self.watchers {
            if task_names.contains(&task.name) {
                return Err(eyre::eyre!(
                    "Task name '{}' is duplicated. Task names must be unique",
                    task.name
                ));
            }
            task_names.insert(task.name.clone());
        }
        for task in &mut self.alerts {
            if task_names.contains(&task.name) {
                return Err(eyre::eyre!(
                    "Task name '{}' is duplicated. Task names must be unique",
                    task.name
                ));
            }
            task_names.insert(task.name.clone());
        }
        Ok(())
    }

    /// Outputs the running Configuration to the default config path
    /// If the path to the file does not exist, it creates all directories
    pub fn create_config_file(&self) -> Result<()> {
        let stringified = serde_yaml::to_string(self)?;
        if let Some(parent) = self.source.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(&self.source, stringified)?;
        Ok(())
    }
}

impl From<PhConfigBuilder> for PhConfig {
    /// Convert from PhConfigBuilder to PhConfig
    fn from(c: PhConfigBuilder) -> PhConfig {
        c.try_build().expect("Can't build config from builder")
    }
}

impl Default for PhConfigBuilder {
    /// Default PhConfigBuilder
    fn default() -> Self {
        Self { builder: Config::builder() }
    }
}

/// Try to convert relevant paths to absolute. It will resolve both
/// the tilde (~) to the $HOME directory, as also `./ and ../`. It doesn't
/// require for the file to exist (like std::fs::canonicalize does).
/// Most likely it won't work in Windows.
/// If the path does not exist, it will return an error.
fn try_absolute_path_of(path: &Path) -> Result<PathBuf> {
    let expanded: PathBuf = if path.starts_with("~") {
        shellexpand::tilde(&path.to_string_lossy()).as_ref().into()
    } else {
        path.to_owned()
    };
    let absolute_path: PathBuf = expanded.absolutize()?.into();
    if absolute_path.exists() {
        Ok(absolute_path)
    } else {
        std::fs::create_dir_all(&absolute_path)?;
        Ok(absolute_path)
    }
}

/// Trigger structure
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trigger {
    pub rule: String,  // Rule of the trigger
    pub interval: u32, // Interval of the trigger
}

/// A wrapper for sensitive values

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::{
        action::{ActionKind, WalletKind},
        watcher::WatcherKind,
    };

    use super::*;
    use std::path::Path;

    #[test]
    fn test_with_canonicalized_paths() {
        let mut config = PhConfig::default();
        config.source = PathBuf::from("./");
        let result = config.with_canonicalized_paths();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.source, Path::new(".").canonicalize().unwrap());
    }

    #[test]
    fn test_check_duplicate_task_names() {
        let mut config = PhConfig::default();
        let action_config = ActionConfig {
            name: "duplicate".to_string(),
            on: Default::default(),
            kind: ActionKind::Evm(Default::default()),
            delay: 0,
        };
        config.actions.push(action_config);
        let watcher_config = WatcherConfig {
            name: "duplicate".to_string(),
            kind: WatcherKind::Evm(Default::default()),
        };
        config.watchers.push(watcher_config);
        let result = config.check_duplicate_task_names();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_config_file() {
        let mut config = PhConfig::default();
        let dir = TempDir::new().expect("Failed to create temp dir");
        config.source = dir.path().join("test_config.yaml");
        let result = config.create_config_file();
        assert!(result.is_ok());
        // TempDir automatically deletes the directory and its content when it goes out of scope
    }

    #[test]
    fn test_try_absolute_path_of() {
        let path = PathBuf::from("./");
        let result = try_absolute_path_of(&path);
        assert!(result.is_ok());
        let abs_path = result.unwrap();
        assert_eq!(abs_path, Path::new(".").canonicalize().unwrap());
    }

    #[test]
    fn test_with_subst_vars_env_not_set() {
        let json = r#"
        {
            "actions": [
                {
                    "name": "testAction",
                    "contractName": "TestContract",
                    "profile": "default",
                    "functionSignature": "testFunction()",
                    "functionArguments": ["arg1", "arg2"],
                    "wallet": "${TEST_WITH_SUBST}",
                    "foundryProjectRootPath": "./",
                    "delay": 0,
                    "on": []
                }
            ]
        }
        "#;
        let config: PhConfig = serde_json::from_str(json).unwrap();
        if let ActionKind::Evm(evm) = &config.actions[0].kind {
            assert_eq!(
                evm.wallet.expose(),
                &WalletKind::PrivateKey("${TEST_WITH_SUBST}".to_string())
            );
        } else {
            panic!("No Evm kind found");
        };
    }

    #[test]
    fn test_with_canonicalized_paths_invalid_path() {
        let mut config = PhConfig::default();
        config.source = PathBuf::from("/path/does/not/exist");
        let result = config.with_canonicalized_paths();
        assert!(result.is_err());
    }

    #[test]
    fn test_h_config_builder_set_override_empty_key() {
        let config_builder = PhConfigBuilder::new().set_override("", "test_value");
        assert!(config_builder.is_err());
    }

    #[test]
    fn test_ph_config_builder_set_override_option_empty_key() {
        let config_builder = PhConfigBuilder::new().set_override_option("", Some("test_value"));
        assert!(config_builder.is_err());
    }
}
