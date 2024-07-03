use phylax_interfaces::{activity::DynActivity, spawnable::IntoActivity};

use crate::components::{
    ActionConfig, ActionMeta, AlertConfig, AlertMeta, NotifConfig, NotifMeta, WatcherConfig,
    WatcherMeta,
};

/// Represents a spawnable alert with metadata and dynamic activity.
pub struct SpawnableAlert {
    /// Metadata associated with the alert.
    pub meta: AlertMeta,
    /// Dynamic activity that can be executed for this alert.
    pub activity: Box<dyn DynActivity>,
}

impl<Ext: IntoActivity> From<AlertConfig<Ext>> for SpawnableAlert {
    fn from(val: AlertConfig<Ext>) -> Self {
        SpawnableAlert { meta: val.meta, activity: val.kind.into_activity() }
    }
}

/// Represents a spawnable action with metadata and dynamic activity.
pub struct SpawnableAction {
    /// Metadata associated with the action.
    pub meta: ActionMeta,
    /// Dynamic activity that can be executed for this action.
    pub activity: Box<dyn DynActivity>,
}

impl<Ext: IntoActivity> From<ActionConfig<Ext>> for SpawnableAction {
    fn from(val: ActionConfig<Ext>) -> Self {
        SpawnableAction { meta: val.meta, activity: val.kind.into_activity() }
    }
}

/// Represents a spawnable watcher with metadata and dynamic activity.
pub struct SpawnableWatcher {
    /// Metadata associated with the watcher.
    pub meta: WatcherMeta,
    /// Dynamic activity that can be executed for this watcher.
    pub activity: Box<dyn DynActivity>,
}

impl<Ext: IntoActivity> From<WatcherConfig<Ext>> for SpawnableWatcher {
    fn from(val: WatcherConfig<Ext>) -> Self {
        SpawnableWatcher { meta: val.meta, activity: val.kind.into_activity() }
    }
}

/// Represents a spawnable notification with metadata and dynamic activity.
pub struct SpawnableNotif {
    /// Metadata associated with the notification.
    pub meta: NotifMeta,
    /// Dynamic activity that can be executed for this notification.
    pub activity: Box<dyn DynActivity>,
}

impl<Ext: IntoActivity> From<NotifConfig<Ext>> for SpawnableNotif {
    fn from(val: NotifConfig<Ext>) -> Self {
        SpawnableNotif { meta: val.meta, activity: val.kind.into_activity() }
    }
}
