use serde::{Deserialize, Serialize};

/// Generic config of an action
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionConfig<Ext> {
    #[serde(flatten)]
    pub meta: ActionMeta,
    /// The type of the action
    #[serde(flatten)]
    pub kind: Ext,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionMeta {
    /// The name of the action
    pub name: String,
    /// The alert names that will trigger this action
    #[serde(default)]
    pub on_alert: Vec<String>,
    /// The minimum amount of time to wait between executions of the action
    /// (in seconds)
    #[serde(default)]
    pub cooldown: u32,
    /// Whether the action is exposed to the API
    #[serde(default)]
    pub exposed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertConfig<Ext> {
    #[serde(flatten)]
    pub meta: AlertMeta,
    #[serde(flatten)]
    pub kind: Ext,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertMeta {
    /// Name of the Alert
    pub name: String,
    #[serde(default)]
    pub on: AlertTrigger,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub enum AlertTrigger {
    #[default]
    StateChange,
    Api,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WatcherConfig<Ext> {
    #[serde(flatten)]
    pub meta: WatcherMeta,
    #[serde(flatten)]
    pub kind: Ext,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WatcherMeta {
    /// The name of the Watcher
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotifConfig<Ext> {
    #[serde(flatten)]
    pub meta: NotifMeta,
    #[serde(flatten)]
    /// The notification kind
    pub kind: Ext,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotifMeta {
    /// The name of the notification
    pub name: String,
    #[serde(flatten)]
    /// The optonal label filter is used to filter out notifications that do not match the label
    /// filter.
    #[serde(default)]
    pub label_filter: Option<String>,
}
