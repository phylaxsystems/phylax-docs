use super::{
    dns::TaskDns,
    error::SpawnError,
    event_buffer::EventBuffer,
    subscriptions::Subscriptions,
    task::{Task, TaskCategory},
};
use crate::{
    activities::{
        system::{metrics::Metrics, orchestrator::Orchestrator},
        Activity,
    },
    events::EventBus,
};
use phylax_api::Api;
use phylax_common::metrics::MetricsRegistry;
use phylax_config::{
    ActionConfig, ActionKind, AlertConfig, ApiConfig, ChatPlatform, MetricsConfig,
    OrchestratorConfig, PhConfig, WatcherConfig, API_TASK_NAME, METRICS_TASK_NAME,
    ORCHESTRATOR_TASK_NAME,
};
use phylax_monitor::{EvmAlert, EvmWatcher};
use phylax_respond::{DiscordClient, EvmAction, GenClient, SlackClient, Webhook};
use phylax_tracing::tracing::{debug, info_span, Span};
use sisyphus_tasks::{Boulder, Sisyphus};
use std::sync::Arc;
use tokio::sync::Mutex;

// Builds a new Task
pub(crate) struct TaskBuilder {
    pub name: String,
    pub id: u32,
    pub span: Span,
    pub metrics: MetricsRegistry,
    pub category: TaskCategory,
    pub event_bus: EventBus,
    pub dns: Arc<TaskDns>,
    pub subscriptions: Subscriptions,
    pub buffer: EventBuffer,
}

/// The artifacts that are produced when a [`Task`] is spawned
pub struct SpawnArtifact {
    /// The handle to the sisyphus wrapper
    pub handle: Sisyphus,
    /// The id of the task
    pub id: u32,
}

/// Implementation of the TaskBuilder struct
impl TaskBuilder {
    /// Creates a new TaskBuilder with default values
    pub fn new() -> Self {
        let name = String::from("new_task");
        let id = 0;
        let span = Span::none();
        let metrics = MetricsRegistry::new();
        let category = TaskCategory::UserDefined;
        let event_bus = EventBus::new();
        let dns = Arc::new(TaskDns::new());
        let subscriptions = Subscriptions::new();
        let buffer = EventBuffer::new();
        Self { name, id, span, metrics, category, event_bus, dns, subscriptions, buffer }
    }

    /// Sets the name of the TaskBuilder
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the id of the TaskBuilder
    pub fn with_id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    /// Sets the span of the TaskBuilder
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    /// Sets the metrics of the TaskBuilder
    pub fn with_metrics(mut self, registry: MetricsRegistry) -> Self {
        self.metrics = registry;
        self
    }

    /// Sets the category of the TaskBuilder
    pub fn with_category(mut self, category: TaskCategory) -> Self {
        self.category = category;
        self
    }

    /// Sets the event bus of the TaskBuilder
    pub fn with_event_bus(mut self, event_bus: EventBus) -> Self {
        self.event_bus = event_bus;
        self
    }

    /// Sets the dns of the TaskBuilder
    pub fn with_dns(mut self, dns: Arc<TaskDns>) -> Self {
        self.dns = dns;
        self
    }

    /// Sets the subscriptions of the TaskBuilder
    pub fn with_subscriptions(mut self, subscriptions: Subscriptions) -> Self {
        self.subscriptions = subscriptions;
        self
    }

    /// Builds a Task from the TaskBuilder
    fn build<A>(self, activity: A) -> Task<A>
    where
        A: Activity,
    {
        Task {
            name: self.name,
            id: self.id,
            span: self.span,
            metrics: self.metrics,
            category: self.category,
            event_bus: self.event_bus,
            dns: self.dns,
            subscriptions: self.subscriptions,
            buffer: self.buffer,
            inner: Arc::new(Mutex::new(activity)),
            maybe_bg_handle: Default::default(),
            maybe_fg_handle: Default::default(),
            fg_working: Default::default(),
            bg_working: Default::default(),
            internal_shutdown_tx: tokio::sync::broadcast::channel(1).0,
        }
    }

    /// Builds and spawns a Task from the TaskBuilder
    pub fn build_and_spawn(
        self,
        activity: impl Activity + 'static,
    ) -> Result<SpawnArtifact, SpawnError> {
        let task = self.build(activity);
        let id = task.id;
        let handle = task.run_until_panic();
        Ok(SpawnArtifact { handle, id })
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A trait that defines the behavior of a spawnable task.
pub trait Spawnable {
    /// Returns the name of the task.
    fn name(&self) -> String;

    /// Returns the category of the task.
    fn category(&self) -> TaskCategory;

    /// Returns the subscriptions of the task.
    fn subscriptions(&self) -> Result<Subscriptions, SpawnError>;

    /// Returns the activity of the task given a context.
    /// We use SpawnableActivities because if we add a Generic, then the trait will not be
    /// Object-safe
    fn get_activity(&self, context: SpawnContext) -> Result<SpawnableActivities, SpawnError>;

    /// Spawns a task with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `bus` - An EventBus instance.
    /// * `dns` - An Arc wrapped TaskDns instance.
    /// * `metrics` - A MetricsRegistry instance.
    /// * `full_config` - An Arc wrapped PhConfig instance.
    ///
    /// # Returns
    ///
    /// * `Result<SpawnArtifact, SpawnError>` - The result of the spawn operation.
    fn spawn_task(
        &self,
        bus: EventBus,
        dns: Arc<TaskDns>,
        metrics: MetricsRegistry,
        full_config: Arc<PhConfig>,
    ) -> Result<SpawnArtifact, SpawnError> {
        let context = SpawnContext::new(metrics.clone(), full_config);
        let name = self.name();
        let id = dns.get_id_from_name(&name);
        let builder = TaskBuilder::new()
            .with_name(self.name())
            .with_id(id)
            .with_span(info_span!(parent: Span::none(), "[Task]", name, id))
            .with_category(self.category())
            .with_subscriptions(self.subscriptions()?)
            .with_event_bus(bus)
            .with_metrics(metrics)
            .with_dns(dns);
        let activity = self.get_activity(context)?;
        debug!(name = self.name(), id, category = ?self.category(), subscriptions = ?self.subscriptions(), activity_type = activity.get_activity_name(), "Spawning Task");
        match activity {
            SpawnableActivities::EvmAction(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::GenWebhook(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::SlackWebhook(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::DiscordWebhook(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::EvmAlert(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::Api(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::EvmWatcher(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::Orchestrator(activity) => builder.build_and_spawn(activity),
            SpawnableActivities::Metrics(activity) => builder.build_and_spawn(activity),
        }
    }
}

/// Enum representing the different types of spawnable activities. This is used to
pub enum SpawnableActivities {
    EvmAction(EvmAction),
    GenWebhook(Webhook<GenClient>),
    SlackWebhook(Webhook<SlackClient>),
    DiscordWebhook(Webhook<DiscordClient>),
    EvmAlert(EvmAlert),
    Api(Api),
    EvmWatcher(EvmWatcher),
    Orchestrator(Orchestrator),
    Metrics(Metrics),
}

impl SpawnableActivities {
    /// Returns the name of the selected activity
    pub fn get_activity_name(&self) -> &'static str {
        match self {
            SpawnableActivities::EvmAction(_) => "EvmAction",
            SpawnableActivities::GenWebhook(_) => "GenWebhook",
            SpawnableActivities::SlackWebhook(_) => "SlackWebhook",
            SpawnableActivities::DiscordWebhook(_) => "DiscordWebhook",
            SpawnableActivities::EvmAlert(_) => "EvmAlert",
            SpawnableActivities::Api(_) => "Api",
            SpawnableActivities::EvmWatcher(_) => "EvmWatcher",
            SpawnableActivities::Orchestrator(_) => "Orchestrator",
            SpawnableActivities::Metrics(_) => "Metrics",
        }
    }
}

/// Struct representing the context in which a task is spawned.
pub struct SpawnContext {
    /// The full configuration for the task.
    full_config: Arc<PhConfig>,
    /// The metrics registry for the task.
    metrics: MetricsRegistry,
}

impl SpawnContext {
    /// Create a new spawn context
    fn new(metrics: MetricsRegistry, full_config: Arc<PhConfig>) -> Self {
        Self { full_config, metrics }
    }
}

impl Spawnable for ActionConfig {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn category(&self) -> TaskCategory {
        TaskCategory::UserDefined
    }

    fn subscriptions(&self) -> Result<Subscriptions, SpawnError> {
        Ok(Subscriptions::try_from_iterator(self.on.clone())?)
    }

    fn get_activity(&self, _context: SpawnContext) -> Result<SpawnableActivities, SpawnError> {
        Ok(match self.kind.clone() {
            ActionKind::GeneralWebhook(config) => SpawnableActivities::GenWebhook(
                Webhook::<GenClient>::try_from(config.clone())
                    .map_err(|e| SpawnError::InvalidConfig(Box::new(config), Box::new(e)))?,
            ),
            ActionKind::ChatWebhook(config) => match config.platform {
                ChatPlatform::Discord => SpawnableActivities::DiscordWebhook(
                    Webhook::<DiscordClient>::try_from(config.clone())
                        .map_err(|e| SpawnError::InvalidConfig(Box::new(config), Box::new(e)))?,
                ),
                ChatPlatform::Slack => SpawnableActivities::SlackWebhook(
                    Webhook::<SlackClient>::try_from(config.clone())
                        .map_err(|e| SpawnError::InvalidConfig(Box::new(config), Box::new(e)))?,
                ),
            },
            ActionKind::Evm(config) => SpawnableActivities::EvmAction(
                EvmAction::try_from(config.clone())
                    .map_err(|e| SpawnError::InvalidConfig(Box::new(config), Box::new(e)))?,
            ),
        })
    }
}

impl Spawnable for AlertConfig {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn category(&self) -> TaskCategory {
        TaskCategory::UserDefined
    }

    fn subscriptions(&self) -> Result<Subscriptions, SpawnError> {
        Ok(Subscriptions::try_from_iterator(self.on.clone())?)
    }

    fn get_activity(&self, _context: SpawnContext) -> Result<SpawnableActivities, SpawnError> {
        match self.kind.clone() {
            phylax_config::AlertKind::Evm(evm) => Ok(SpawnableActivities::EvmAlert(evm.into())),
        }
    }
}

impl Spawnable for WatcherConfig {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn category(&self) -> TaskCategory {
        TaskCategory::UserDefined
    }

    fn subscriptions(&self) -> Result<Subscriptions, SpawnError> {
        Ok(Subscriptions::new())
    }

    fn get_activity(&self, _context: SpawnContext) -> Result<SpawnableActivities, SpawnError> {
        match self.kind.clone() {
            phylax_config::WatcherKind::Evm(evm) => {
                let evm_watcher = evm
                    .clone()
                    .try_into()
                    .map_err(|e| SpawnError::InvalidConfig(Box::new(evm), Box::new(e)))?;
                Ok(SpawnableActivities::EvmWatcher(evm_watcher))
            }
        }
    }
}

impl Spawnable for OrchestratorConfig {
    fn name(&self) -> String {
        phylax_config::ORCHESTRATOR_TASK_NAME.to_string()
    }

    fn category(&self) -> TaskCategory {
        TaskCategory::System
    }

    fn subscriptions(&self) -> Result<Subscriptions, SpawnError> {
        Ok(Subscriptions::try_from_iterator(self.on.clone())?)
    }

    fn get_activity(&self, context: SpawnContext) -> Result<SpawnableActivities, SpawnError> {
        let orchestrator = Orchestrator::new(context.full_config.clone());
        Ok(SpawnableActivities::Orchestrator(orchestrator))
    }
}

impl Spawnable for ApiConfig {
    fn name(&self) -> String {
        phylax_config::API_TASK_NAME.to_string()
    }

    fn category(&self) -> TaskCategory {
        TaskCategory::System
    }

    fn subscriptions(&self) -> Result<Subscriptions, SpawnError> {
        Ok(Subscriptions::try_from_iterator(self.on.clone())?)
    }

    fn get_activity(&self, context: SpawnContext) -> Result<SpawnableActivities, SpawnError> {
        let api = Api::new(self.bind_ip, context.metrics.clone(), context.full_config.clone());
        Ok(SpawnableActivities::Api(api))
    }
}

impl Spawnable for MetricsConfig {
    fn name(&self) -> String {
        phylax_config::METRICS_TASK_NAME.to_string()
    }

    fn category(&self) -> TaskCategory {
        TaskCategory::System
    }

    fn subscriptions(&self) -> Result<Subscriptions, SpawnError> {
        Ok(Subscriptions::try_from_iterator(self.on.clone())?)
    }

    fn get_activity(&self, _context: SpawnContext) -> Result<SpawnableActivities, SpawnError> {
        let metrics = Metrics::new();
        Ok(SpawnableActivities::Metrics(metrics))
    }
}

/// Spawns a task from the configuration if the task name matches with any of the watchers, alerts,
/// or actions. Then it checks for a match against system tasks. It Returns the artifact of the
/// spawned task or a [`SpawnError`] if no match is found.
pub fn spawn_task_of_config(
    task_name: String,
    config: Arc<PhConfig>,
) -> Result<SpawnArtifact, SpawnError> {
    let mut artifact: Option<SpawnArtifact> = None;
    let bus = EventBus::default();
    let dns = Arc::new(TaskDns::default());
    let metrics = MetricsRegistry::default();

    if artifact.is_none() {
        for alert in &config.alerts {
            if alert.name == task_name {
                artifact = Some(alert.clone().spawn_task(
                    bus.get_copy(),
                    dns.clone(),
                    metrics.clone(),
                    config.clone(),
                )?);
                break;
            }
        }
    }

    if artifact.is_none() {
        for action in &config.actions {
            if action.name == task_name {
                artifact = Some(action.clone().spawn_task(
                    bus.get_copy(),
                    dns.clone(),
                    metrics.clone(),
                    config.clone(),
                )?);
                break;
            }
        }
    }
    if artifact.is_none() {
        if API_TASK_NAME == task_name {
            artifact = Some(config.api.spawn_task(
                bus.get_copy(),
                dns.clone(),
                metrics.clone(),
                config.clone(),
            )?);
        } else if ORCHESTRATOR_TASK_NAME == task_name {
            artifact = Some(config.orchestrator.spawn_task(
                bus.get_copy(),
                dns.clone(),
                metrics.clone(),
                config.clone(),
            )?);
        } else if METRICS_TASK_NAME == task_name {
            artifact = Some(config.metrics.spawn_task(
                bus.get_copy(),
                dns.clone(),
                metrics.clone(),
                config.clone(),
            )?);
        }
    }

    artifact.ok_or(SpawnError::TaskNotFound(task_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use phylax_config::{
        ActionConfig, AlertKind, EvmAlertConfig, EvmWatcherConfig, GeneralWebhookConfig,
        SensitiveValue, WatcherKind, WebhookAuth,
    };
    use std::{path::PathBuf, sync::Arc};
    use url::Url;

    #[test]
    fn test_task_builder() {
        let task_builder = TaskBuilder::new()
            .with_name("test_task")
            .with_id(1)
            .with_span(Span::none())
            .with_metrics(MetricsRegistry::new())
            .with_category(TaskCategory::UserDefined)
            .with_event_bus(EventBus::new())
            .with_dns(Arc::new(TaskDns::new()))
            .with_subscriptions(Subscriptions::new());

        assert_eq!(task_builder.name, "test_task");
        assert_eq!(task_builder.id, 1);
    }

    #[test]
    fn test_spawnable_for_action_config() {
        let action_config = ActionConfig {
            name: "test_action".to_string(),
            on: vec![],
            kind: ActionKind::GeneralWebhook(GeneralWebhookConfig {
                target: "http://target.com".parse().unwrap(),
                method: "POST".parse().unwrap(),
                payload: "".to_owned(),
                authentication: SensitiveValue::new(WebhookAuth::None),
            }),
            delay: 0,
        };

        assert_eq!(action_config.name(), "test_action".to_string());
        assert_eq!(action_config.category(), TaskCategory::UserDefined);

        let subscriptions = action_config.subscriptions();
        assert!(subscriptions.is_ok());

        let context = SpawnContext::new(MetricsRegistry::new(), Arc::new(PhConfig::default()));
        let activity = action_config.get_activity(context);
        assert!(activity.is_ok());
        match activity.unwrap() {
            SpawnableActivities::GenWebhook(_) => (),
            _ => panic!("Expected GenWebhook"),
        }
    }

    #[test]
    fn test_spawnable_for_alert_config() {
        let alerts_root: PathBuf = "./alerts".parse().unwrap();
        let evm = EvmAlertConfig {
            foundry_project_root_path: alerts_root.clone(),
            foundry_profile: "default".to_string(),
        };
        let alert_config = AlertConfig {
            name: "test_alert".to_string(),
            on: vec![],
            kind: AlertKind::Evm(evm),
            delay: 0,
        };

        assert_eq!(alert_config.name(), "test_alert".to_string());
        assert_eq!(alert_config.category(), TaskCategory::UserDefined);

        let subscriptions = alert_config.subscriptions();
        assert!(subscriptions.is_ok());

        let context = SpawnContext::new(MetricsRegistry::new(), Arc::new(PhConfig::default()));
        let activity = alert_config.get_activity(context);
        assert!(activity.is_ok());
        match activity.unwrap() {
            SpawnableActivities::EvmAlert(alert) => {
                assert_eq!(alert.foundry_config_root, alerts_root);
            }
            _ => panic!("Expected EvmAlert"),
        }
    }

    #[test]
    fn test_spawnable_for_watcher_config() {
        let rpc = "http://test_rpc.gr".to_string();
        let chain = ethers_core::types::Chain::AnvilHardhat;
        let evm_watcher_config =
            EvmWatcherConfig { chain, eth_rpc: SensitiveValue::new(rpc.clone()) };
        let watcher_config = WatcherConfig {
            name: "test_watcher".to_string(),
            kind: WatcherKind::Evm(evm_watcher_config),
        };

        assert_eq!(watcher_config.name(), "test_watcher".to_string());
        assert_eq!(watcher_config.category(), TaskCategory::UserDefined);

        let subscriptions = watcher_config.subscriptions();
        assert!(subscriptions.is_ok());

        let context = SpawnContext::new(MetricsRegistry::new(), Arc::new(PhConfig::default()));
        let activity = watcher_config.get_activity(context);
        assert!(activity.is_ok());
        match activity.unwrap() {
            SpawnableActivities::EvmWatcher(watcher) => {
                assert_eq!(watcher.chain, chain);
                assert_eq!(watcher.endpoint, Url::parse(&rpc).unwrap())
            }
            _ => panic!("Expected EvmWatcher"),
        }
    }

    #[test]
    fn test_spawn_task_of_config() {
        let config = Arc::new(PhConfig::default());
        let task_name = "test_task".to_string();

        let result = spawn_task_of_config(task_name, config);
        assert!(result.is_err());
    }
    #[test]
    fn test_spawnable_for_orchestrator_config() {
        let orchestrator_config = OrchestratorConfig { on: vec![] };

        assert_eq!(orchestrator_config.name(), phylax_config::ORCHESTRATOR_TASK_NAME.to_string());
        assert_eq!(orchestrator_config.category(), TaskCategory::System);

        let subscriptions = orchestrator_config.subscriptions();
        assert!(subscriptions.is_ok());

        let context = SpawnContext::new(MetricsRegistry::new(), Arc::new(PhConfig::default()));
        let activity = orchestrator_config.get_activity(context);
        assert!(activity.is_ok());
        match activity.unwrap() {
            SpawnableActivities::Orchestrator(_) => (),
            _ => panic!("Expected Orchestrator"),
        }
    }

    #[test]
    fn test_spawnable_for_api_config() {
        let api_config = ApiConfig {
            bind_ip: "127.0.0.1:8080".parse().unwrap(),
            on: vec![],
            enable_api: true,
            enable_metrics: true,
            jwt_secret: None,
        };

        assert_eq!(api_config.name(), phylax_config::API_TASK_NAME.to_string());
        assert_eq!(api_config.category(), TaskCategory::System);

        let subscriptions = api_config.subscriptions();
        assert!(subscriptions.is_ok());

        let context = SpawnContext::new(MetricsRegistry::new(), Arc::new(PhConfig::default()));
        let activity = api_config.get_activity(context);
        assert!(activity.is_ok());
        match activity.unwrap() {
            SpawnableActivities::Api(api) => {
                assert_eq!(api.bind_ip.to_string(), "127.0.0.1:8080");
            }
            _ => panic!("Expected Api"),
        }
    }

    #[test]
    fn test_spawnable_for_metrics_config() {
        let metrics_config = MetricsConfig { on: vec![] };

        assert_eq!(metrics_config.name(), phylax_config::METRICS_TASK_NAME.to_string());
        assert_eq!(metrics_config.category(), TaskCategory::System);

        let subscriptions = metrics_config.subscriptions();
        assert!(subscriptions.is_ok());

        let context = SpawnContext::new(MetricsRegistry::new(), Arc::new(PhConfig::default()));
        let activity = metrics_config.get_activity(context);
        assert!(activity.is_ok());
        match activity.unwrap() {
            SpawnableActivities::Metrics(_) => (),
            _ => panic!("Expected Metrics"),
        }
    }
}
