use crate::{
    components::{ActionConfig, AlertConfig, NotifConfig, WatcherConfig},
    error::ConfigError,
    spawnable::{SpawnableAction, SpawnableAlert, SpawnableNotif, SpawnableWatcher},
};
use phylax_interfaces::spawnable::IntoActivity;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashSet, ffi::OsStr, path::Path};
use toml;

pub const EXTENSION: &str = "toml";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NoExt;

/// Main configuration structure
/// This structure holds the configurations for alerts, watchers, actions, and notifications.
/// It ensures that unknown fields are not allowed in the configuration to prevent errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config<Al = NoExt, W = NoExt, Ac = NoExt, N = NoExt> {
    /// The configurations of the alerts. Defaults to an empty vector if no alerts are specified.
    #[serde(default = "default_vec")]
    pub alerts: Vec<AlertConfig<Al>>,
    /// The configurations of the watchers.
    #[serde(default = "default_vec")]
    pub watchers: Vec<WatcherConfig<W>>,
    /// The configurations of the actions.
    #[serde(default = "default_vec")]
    pub actions: Vec<ActionConfig<Ac>>,
    /// The configurations of the notifications.
    #[serde(default = "default_vec")]
    pub notifications: Vec<NotifConfig<N>>,
}

/// Helper implementation of the default Config without
/// any extensions
impl<Al, W, Ac, N> Config<Al, W, Ac, N>
where
    Al: DeserializeOwned + Serialize + Clone,
    W: DeserializeOwned + Serialize + Clone,
    Ac: DeserializeOwned + Serialize + Clone,
    N: DeserializeOwned + Serialize + Clone,
{
    /// Creates a `PhConfig` instance from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Load the configuration from a TOML file at the given path.
    ///
    /// # Arguments
    /// * `path` - A reference to the path of the TOML file.
    ///
    /// # Returns
    /// A result containing the loaded configuration or an error if the file could not be read or
    /// parsed.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if path.extension() != Some(OsStr::new(EXTENSION)) {
            return Err(ConfigError::WrongConfigFileExtension);
        }

        let toml_string = std::fs::read_to_string(path).map_err(ConfigError::FileReadError)?;

        let config = Self::from_toml(&toml_string).map_err(ConfigError::FailedToParseToml)?;
        config.validate_action_alerts()?;
        config.validate_unique_names()?;
        // TODO(odysseas): validate notification filter syntax, best as part of the deserialization
        Ok(config)
    }

    /// Save the configuration to toml file.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if path.extension() != Some(OsStr::new(EXTENSION)) {
            return Err(ConfigError::WrongConfigFileExtension);
        }
        let toml_string = toml::to_string(self)?;
        std::fs::write(path, toml_string).map_err(ConfigError::FileWriteError)
    }

    /// Validates that all configuration struct names are unique across alerts, watchers, actions,
    /// and notifications.
    ///
    /// # Returns
    /// A result indicating whether the validation passed or an error message detailing the issue.
    pub fn validate_unique_names(&self) -> Result<(), ConfigError> {
        let mut name_set = HashSet::new();

        for (index, alert) in self.alerts.iter().enumerate() {
            if !name_set.insert(alert.meta.name.clone()) {
                return Err(ConfigError::DuplicateName(
                    alert.meta.name.clone(),
                    "alerts".to_string(),
                    index,
                ));
            }
        }
        for (index, watcher) in self.watchers.iter().enumerate() {
            if !name_set.insert(watcher.meta.name.clone()) {
                return Err(ConfigError::DuplicateName(
                    watcher.meta.name.clone(),
                    "watchers".to_string(),
                    index,
                ));
            }
        }

        for (index, action) in self.actions.iter().enumerate() {
            if !name_set.insert(action.meta.name.clone()) {
                return Err(ConfigError::DuplicateName(
                    action.meta.name.clone(),
                    "actions".to_string(),
                    index,
                ));
            }
        }

        for (index, notification) in self.notifications.iter().enumerate() {
            if !name_set.insert(notification.meta.name.clone()) {
                return Err(ConfigError::DuplicateName(
                    notification.meta.name.clone(),
                    "notifications".to_string(),
                    index,
                ));
            }
        }

        Ok(())
    }

    /// Validates that all `on_alert` names in `ActionConfig` are valid alert names defined in the
    /// configuration.
    ///
    /// # Arguments
    /// * `alerts` - A reference to the vector of alert configurations.
    ///
    /// # Returns
    /// A result indicating whether the validation passed or an error message detailing the issue.
    pub fn validate_action_alerts(&self) -> Result<(), ConfigError> {
        let valid_alert_names: HashSet<String> =
            self.alerts.iter().map(|alert| alert.meta.name.clone()).collect();

        for action in &self.actions {
            for alert_name in &action.meta.on_alert {
                if !valid_alert_names.contains(alert_name) {
                    return Err(ConfigError::InvalidAlertName(
                        alert_name.to_string(),
                        action.meta.name.to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Retrieves all spawnable alerts from the configuration.
impl<Al, W, Ac, N> Config<Al, W, Ac, N>
where
    Al: Clone + IntoActivity,
    W: Clone,
    Ac: Clone,
    N: Clone,
{
    pub fn get_spawnable_alerts(&self) -> Vec<SpawnableAlert> {
        self.alerts.clone().into_iter().map(|alert| alert.into()).collect()
    }
}

/// Retrieves all spawnable watchers from the configuration.
impl<Al, W, Ac, N> Config<Al, W, Ac, N>
where
    Al: Clone,
    W: Clone + IntoActivity,
    Ac: Clone,
    N: Clone,
{
    pub fn get_spawnable_watchers(&self) -> Vec<SpawnableWatcher> {
        self.watchers.clone().into_iter().map(|watcher| watcher.into()).collect()
    }
}

/// Retrieves all spawnable actions from the configuration.
impl<Al, W, Ac, N> Config<Al, W, Ac, N>
where
    Al: Clone,
    W: Clone,
    Ac: Clone + IntoActivity,
    N: Clone,
{
    pub fn get_spawnable_actions(&self) -> Vec<SpawnableAction> {
        self.actions.clone().into_iter().map(|action| action.into()).collect()
    }
}

/// Retrieves all spawnable notifications from the configuration.
impl<Al, W, Ac, N> Config<Al, W, Ac, N>
where
    Al: Clone,
    W: Clone,
    Ac: Clone,
    N: Clone + IntoActivity,
{
    pub fn get_spawnable_notifications(&self) -> Vec<SpawnableNotif> {
        self.notifications.clone().into_iter().map(|notif| notif.into()).collect()
    }
}
// we need this because if we use the deault implementation, it will require Ext to have a
// default implementation as well
pub fn default_vec<T>() -> Vec<T> {
    vec![]
}
#[cfg(test)]
mod tests {
    use super::*;
    use phylax_interfaces::{
        activity::{Activity, DynActivity},
        context::ActivityContext,
        error::PhylaxError,
        message::MessageDetails,
    };
    use phylax_tasks::activity::ActivityCell;
    use phylax_test_utils::mock_activity;
    use serde::Deserialize;
    use tempfile::tempdir;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct AlertConfig {
        alert_path: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct WatcherConfig {
        chain: String,
        endpoints: Vec<String>,
        strategy: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct ActionConfig {
        wallet: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(untagged)]
    enum Alerts {
        AlertConfig(AlertConfig),
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(untagged)]
    enum Watchers {
        WatcherConfig(WatcherConfig),
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(untagged)]
    enum Actions {
        ActionConfig(ActionConfig),
    }
    /// Example of a custom watcher kind that can be used for deserialization.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct CustomWatcherConfig {
        custom_field: String,
    }

    /// Example of a custom watcher kind that can be used for deserialization.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct CustomWatcherConfig2 {
        custom_field_2: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(untagged)]
    enum ExtendedWatchers {
        Watchers(Watchers),
        CustomWatcherConfig(CustomWatcherConfig),
        CustomWatcherConfig2(CustomWatcherConfig2),
    }

    #[test]
    fn test_extended_watchers_parsing() {
        let toml_str = r#"
            [[watchers]]
            name = "Custom Watcher"
            custom_field = "Custom Data"

            [[watchers]]
            name = "Custom Watcher2"
            custom_field_2 = "More Custom Data"

            [[watchers]]
            name = "Custom Watcher3"
            chain = "mainnet"
            endpoints = ["http://localhost:9933"]
            strategy = "asf"
        "#;

        let config: Result<Config<NoExt, ExtendedWatchers, NoExt, NoExt>, _> =
            Config::from_toml(toml_str);

        assert!(config.is_ok(), "Failed to parse ExtendedWatchers from TOML");

        let config = config.unwrap();
        assert_eq!(config.watchers.len(), 3, "Expected two watchers to be parsed");
    }

    #[test]
    fn test_load_invalid_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid_extension.json");

        let config = Config::<Alerts, Watchers, Actions>::load(&file_path);
        assert!(config.is_err());
        match config {
            Err(ConfigError::WrongConfigFileExtension) => (),
            _ => panic!("Expected ConfigError::WrongConfigFileExtension"),
        }
    }

    #[test]
    fn test_save_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("save_config.toml");
        let config = Config::<Alerts, Watchers, Actions> {
            alerts: vec![],
            watchers: vec![],
            actions: vec![],
            notifications: vec![],
        };

        let save_result = config.save(&file_path);
        assert!(save_result.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_validate_action_alerts() {
        let toml_str = r#"
            [[alerts]]
            name = "High Gas Usage"
            alert_path = "/path/to/alert/config"
            
            [[alerts]]
            name = "OP critical"
            alert_path = "/path/to/alert/config"

            [[actions]]
            name = "Restart Node"
            on_alert = ["Gas Usage"]
            wallet = "0x1234567890abcdef"
        "#;
        let config = Config::<Alerts, Watchers, Actions>::from_toml(toml_str).unwrap();
        let validation = config.validate_action_alerts();
        assert!(validation.is_err());
        match validation {
            Err(ConfigError::InvalidAlertName(alert_name, action_name)) => {
                assert_eq!(alert_name, "Gas Usage");
                assert_eq!(action_name, "Restart Node");
            }
            _ => panic!("Expected ConfigError::InvalidAlertName"),
        }
    }

    #[test]
    fn test_validate_unique_names() {
        let toml_str = r#"
            [[alerts]]
            name = "High Gas Usage"
            alert_path = "/path/to/alert/config"
            
            [[alerts]]
            name = "High Gas Usage"
            alert_path = "/path/to/alert/config"

            [[actions]]
            name = "Restart Node"
            on_alert = ["Gas Usage"]
            wallet = "0x1234567890abcdef"
        "#;
        let config = Config::<Alerts, Watchers, Actions>::from_toml(toml_str).unwrap();

        let validation = config.validate_unique_names();
        assert!(validation.is_err());
        match validation {
            Err(ConfigError::DuplicateName(name, kind, index)) => {
                assert_eq!(name, "High Gas Usage");
                assert_eq!(kind, "alerts");
                assert_eq!(index, 1);
            }
            _ => panic!("Expected ConfigError::DuplicateName"),
        }
    }
    struct MockAlert;
    struct MockAction;
    mock_activity!(MockAction, (), (), self _input _context {
        Ok(None)
    });
    mock_activity!(MockAlert, (), (), self _input _context {
        Ok(None)
    });

    impl IntoActivity for Alerts {
        fn into_activity(self) -> Box<(dyn DynActivity)> {
            Box::new(ActivityCell::new("MockAlert", MockAlert))
        }
    }

    impl IntoActivity for Actions {
        fn into_activity(self) -> Box<(dyn DynActivity)> {
            Box::new(ActivityCell::new("MockAction", MockAction))
        }
    }

    #[test]
    fn test_get_spawnable() {
        let toml_str = r#"
            [[alerts]]
            name = "High Gas Usage"
            alert_path = "/path/to/alert/config"
            
            [[alerts]]
            name = "High Gas Usage"
            alert_path = "/path/to/alert/config"

            [[actions]]
            name = "Restart Node"
            on_alert = ["Gas Usage"]
            wallet = "0x1234567890abcdef"
        "#;
        let config = Config::<Alerts, NoExt, Actions>::from_toml(toml_str).unwrap();
        let spawnable_alerts = config.get_spawnable_alerts();
        let spawnable_actions = config.get_spawnable_actions();
        assert_eq!(spawnable_alerts.len(), 2);
        assert_eq!(spawnable_actions.len(), 1);
    }
}
