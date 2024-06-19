use super::{
    error::OrchestratorError,
    events::TaskStatusEventBody,
    task_controller::{TaskController, TaskManager},
    task_handles::TaskHandles,
};
use crate::{
    activities::{Activity, ActivityContext, FgWorkType},
    events::{Event, EventBuilder, EventBus, EventType},
    tasks::{
        dns::TaskDns,
        error::SpawnError,
        spawn::{SpawnArtifact, Spawnable},
    },
};
use async_trait::async_trait;
use eyre::Result;
use flume::r#async::RecvStream;
use phylax_common::metrics::MetricsRegistry;
use phylax_config::PhConfig;
use phylax_tracing::tracing::{info, info_span, Instrument};
use sisyphus_tasks::sisyphus::TaskStatus;
use std::{fmt, sync::Arc};
use tokio::{sync::oneshot, task::JoinHandle};

/// The Orchestrator of tasks. The struct holds the running configuration and it's state
pub struct Orchestrator {
    /// The main configuration of the node
    pub config: Arc<PhConfig>,
    /// The task controller of the orchestrator
    pub task_controller: Option<TaskController>,
}

impl Orchestrator {
    /// Creates a new Orchestrator with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - An Arc wrapped PhConfig that contains the configuration for the Orchestrator.
    pub fn new(config: Arc<PhConfig>) -> Self {
        Self { config, task_controller: Default::default() }
    }

    /// Returns a clone of the current configuration.
    fn get_config(&self) -> Arc<PhConfig> {
        self.config.clone()
    }

    /// Spawns system and user tasks.
    ///
    /// # Arguments
    ///
    /// * `dns` - An Arc wrapped [TaskDns].
    /// * `metrics` - A reference to metrics registry [MetricsRegistry] of the orchestrator.
    /// * `bus` - A reference to the [EventBus] of the orchestrator.
    pub(crate) fn spawn_tasks(
        &mut self,
        dns: &Arc<TaskDns>,
        metrics: &MetricsRegistry,
        bus: &EventBus,
    ) -> Result<TaskHandles, OrchestratorError> {
        let mut artifacts = Vec::new();
        let span = info_span!("tasks spawn");
        let _guard = span.enter();
        let user_tasks = self.spawn_user_tasks(dns, metrics, bus)?;
        let system_tasks = self.spawn_system_tasks(dns, metrics, bus)?;
        artifacts.extend(user_tasks);
        artifacts.extend(system_tasks);
        let handles = self.extract_task_handles(artifacts)?;
        Ok(handles)
    }

    /// Spawns system tasks based on the current configuration.
    ///
    /// # Arguments
    ///
    /// * `dns` - An Arc wrapped [TaskDns].
    /// * `metrics` - A reference to metrics registry [MetricsRegistry] of the orchestrator.
    /// * `bus` - A reference to the [EventBus] of the orchestrator.
    pub(crate) fn spawn_system_tasks(
        &mut self,
        dns: &Arc<TaskDns>,
        metrics: &MetricsRegistry,
        bus: &EventBus,
    ) -> Result<Vec<Result<SpawnArtifact, SpawnError>>, OrchestratorError> {
        let mut all_configs: Vec<Box<dyn Spawnable>> = Vec::new();
        if self.config.api.enable_api {
            info!("API or Metrics endpoint enabled. Spawning task..");
            all_configs.push(Box::new(self.config.api.clone()));
        }
        all_configs.push(Box::new(self.config.metrics.clone()));
        Ok(self.spawn_configs(all_configs, dns, metrics, bus))
    }

    /// Spawns user tasks based on the current configuration.
    ///
    /// # Arguments
    ///
    /// * `dns` - An Arc wrapped [TaskDns].
    /// * `metrics` - A reference to metrics registry [MetricsRegistry] of the orchestrator.
    /// * `bus` - A reference to the [EventBus] of the orchestrator.
    pub(crate) fn spawn_user_tasks(
        &mut self,
        dns: &Arc<TaskDns>,
        metrics: &MetricsRegistry,
        bus: &EventBus,
    ) -> Result<Vec<Result<SpawnArtifact, SpawnError>>, OrchestratorError> {
        let mut all_configs: Vec<Box<dyn Spawnable>> = Vec::new();
        all_configs.extend(
            self.config.actions.iter().map(|config| Box::new(config.clone()) as Box<dyn Spawnable>),
        );
        all_configs.extend(
            self.config
                .watchers
                .iter()
                .map(|config| Box::new(config.clone()) as Box<dyn Spawnable>),
        );
        all_configs.extend(
            self.config.alerts.iter().map(|config| Box::new(config.clone()) as Box<dyn Spawnable>),
        );
        Ok(self.spawn_configs(all_configs, dns, metrics, bus))
    }

    /// Spawn a vector of configs that implement the trait [Spawnable]
    pub(crate) fn spawn_configs(
        &self,
        configs: Vec<Box<dyn Spawnable>>,
        dns: &Arc<TaskDns>,
        metrics: &MetricsRegistry,
        bus: &EventBus,
    ) -> Vec<Result<SpawnArtifact, SpawnError>> {
        configs
            .into_iter()
            .map(|config| {
                config.spawn_task(bus.get_copy(), dns.clone(), metrics.clone(), self.get_config())
            })
            .collect()
    }

    /// Build the [`TaskController`]
    pub(crate) fn build_task_controller(&self, handles: TaskHandles) -> TaskController {
        let (status_tx, status_rx) = flume::unbounded();
        let (shutdown, shutdown_rx) = oneshot::channel();
        let (shutdown_cb, shutdown_cb_rx) = oneshot::channel();
        let manager = TaskManager::new(handles, shutdown_rx, shutdown_cb, status_tx);
        TaskController::new(manager, status_rx, shutdown, shutdown_cb_rx)
    }

    /// Create a [`TaskHandles`] out of an iterator of [SpawnArtifact]
    pub(crate) fn extract_task_handles(
        &self,
        artifacts: impl IntoIterator<Item = Result<SpawnArtifact, SpawnError>>,
    ) -> Result<TaskHandles, OrchestratorError> {
        let mut tasks = TaskHandles::new();
        for artifact in artifacts {
            let artifact = artifact?;
            tasks.insert(artifact.id, artifact.handle);
        }
        Ok(tasks)
    }

    /// Build an event that signifies that the status ([TaskStatus]) of a task changed
    fn task_status_event(&self, name: impl ToString, status: TaskStatus) -> Event {
        EventBuilder::new()
            .with_type(EventType::TaskStatusChange)
            .with_body(TaskStatusEventBody::new(name.to_string(), status).as_value())
            .build()
    }
}

#[async_trait]
impl Activity for Orchestrator {
    type Command = ();
    type InternalStream = RecvStream<'static, Self::InternalStreamItem>;
    type InternalStreamItem = (u32, TaskStatus);

    /// Bootstrap the Orchestrator and spawn all the tasks. Then, have the
    /// [`super::task_controller::TaskController`] monitor any change to their status
    async fn bootstrap(&mut self, context: Arc<ActivityContext>) -> Result<()> {
        let handles = self.spawn_tasks(&context.dns, &context.metrics, &context.bus)?;
        self.task_controller = Some(self.build_task_controller(handles));
        Ok(())
    }

    /// The Orchestator doesn't process any system events
    fn work_type(&self) -> FgWorkType {
        FgWorkType::None
    }

    /// Cleanup the orchestrator by shutting down all the tasks
    async fn cleanup(mut self) -> Result<()> {
        phylax_tracing::tracing::debug!("Shutting down tasks..");
        if let Some(mut controller) = self.task_controller.take() {
            if let Some(tx) = controller.shutdown_tx.take() {
                match tx.send(()) {
                    Ok(_) => (),
                    Err(_err) => return Err(OrchestratorError::TaskShutdownSignal.into()),
                };
            }
            controller.shutdown_cb_rx.take().unwrap().await?;
        }
        Ok(())
    }

    /// An internal stream of (i32, TaskStatus) that originates from the
    /// [`super::task_controller::TaskController`]
    async fn internal_stream(&self) -> Option<Self::InternalStream> {
        self.task_controller.as_ref().map(|controller| controller.status_rx.clone().into_stream())
    }

    /// Process the change of the status of a Task and return an event
    async fn internal_stream_work(
        &mut self,
        item: Self::InternalStreamItem,
        ctx: Arc<ActivityContext>,
    ) -> Option<Event> {
        let (id, status) = item;
        let name = ctx.dns.find_name_from_id(id).unwrap_or("Unknown Task".to_string());
        Some(self.task_status_event(name, status))
    }

    /// Spawn the background work. The background work of the Orchestrator is a [`tokio::task`]
    /// executing the main loop of [`super::task_controller::TaskController`]
    fn spawn_bg_work(&mut self) -> Option<JoinHandle<Result<()>>> {
        let span = info_span!("background work");
        if let Some(controller) = self.task_controller.as_mut() {
            if let Some(manager) = controller.task_manager.take() {
                return Some(tokio::task::spawn(
                    async move {
                        manager.main_loop().await?;
                        Ok(())
                    }
                    .instrument(span),
                ));
            }
        }
        None
    }
}

impl fmt::Display for Orchestrator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Orchestrator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        activities::system::orchestrator::InternalTaskStatus,
        mocks::mock_context::mock_activity_context,
    };
    use phylax_config::{ApiConfig, MetricsConfig, API_TASK_NAME, METRICS_TASK_NAME};
    use std::{sync::Arc, time::Duration};

    fn setup() -> Orchestrator {
        let config = PhConfig::default();
        Orchestrator::new(Arc::new(config))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_spawn_configs() {
        let orchestrator = setup();
        let dns = Arc::new(TaskDns::new());
        let metrics = MetricsRegistry::new();
        let bus = EventBus::new();

        let configs: Vec<Box<dyn Spawnable>> =
            vec![Box::<ApiConfig>::default(), Box::<MetricsConfig>::default()];

        let artifacts = orchestrator.spawn_configs(configs, &dns, &metrics, &bus);
        tokio::time::sleep(Duration::from_millis(500)).await;
        for artifact in artifacts {
            let inner = artifact.unwrap();
            assert_ne!(inner.id, 0);
            assert_eq!(inner.handle.status().to_string(), TaskStatus::Running.to_string());
        }
    }

    #[tokio::test]
    async fn test_build_task_controller() {
        // Arrange
        let orchestrator = setup();

        let handles = TaskHandles::default();
        let controller = orchestrator.build_task_controller(handles);
        // Assert
        assert!(controller.task_manager.is_some());
    }

    #[tokio::test]
    async fn test_spawn_system_tasks_and_extract_task_handles() {
        let mut orchestrator = setup();
        let dns = Arc::new(TaskDns::new());
        let metrics = MetricsRegistry::new();
        let bus = EventBus::new();
        let artifacts = orchestrator.spawn_system_tasks(&dns, &metrics, &bus).unwrap();
        let task_handles = orchestrator.extract_task_handles(artifacts).unwrap();
        let id = dns.get_id_from_name(METRICS_TASK_NAME);
        assert_eq!(task_handles.len(), 1);
        assert!(task_handles.get(&id).is_some());
    }

    #[tokio::test]
    async fn test_task_status_event() {
        let orchestrator = setup();
        let event = orchestrator.task_status_event("TestTask".to_string(), TaskStatus::Running);
        assert_eq!(event.get_type(), &EventType::TaskStatusChange);
        let body: TaskStatusEventBody =
            serde_json::from_value(event.get_body_ref().to_owned()).unwrap();
        assert_eq!(body.task, "TestTask");
        assert_eq!(body.state, InternalTaskStatus::Running);
    }

    #[tokio::test]
    async fn test_bootstrap() {
        let mut config = PhConfig::default();
        config.api.enable_api = true;
        let mut orchestrator = Orchestrator::new(Arc::new(config));
        let context = mock_activity_context(None);
        assert!(orchestrator.bootstrap(context.clone()).await.is_ok());
        let manager = orchestrator.task_controller.unwrap().task_manager.unwrap();
        assert_eq!(manager.tasks.len(), 2);
        let metrics = context.dns.get_id_from_name(METRICS_TASK_NAME);
        let api = context.dns.get_id_from_name(API_TASK_NAME);
        assert!(manager.tasks.get(&metrics).is_some());
        assert!(manager.tasks.get(&api).is_some());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let orchestrator = setup();
        assert!(orchestrator.cleanup().await.is_ok());
    }

    #[tokio::test]
    async fn test_work_type() {
        let orchestrator = setup();
        assert_eq!(orchestrator.work_type(), FgWorkType::None);
    }

    #[tokio::test]
    async fn test_internal_stream() {
        let mut orchestrator = setup();
        assert!(orchestrator.internal_stream().await.is_none());
        let context = mock_activity_context(None);
        orchestrator.bootstrap(context).await.unwrap();
        assert!(orchestrator.internal_stream().await.is_some());
    }

    #[tokio::test]
    async fn test_internal_stream_work() {
        let mut orchestrator = setup();
        let context = mock_activity_context(None);
        let item = (1, TaskStatus::Running);
        let event = orchestrator.internal_stream_work(item.clone(), context).await.unwrap();
        assert_eq!(event, orchestrator.task_status_event("Unknown Task", item.1));
    }

    #[tokio::test]
    async fn test_spawn_bg_work() {
        let mut orchestrator = setup();
        assert!(orchestrator.spawn_bg_work().is_none());
        let context = mock_activity_context(None);
        orchestrator.bootstrap(context).await.unwrap();
        assert!(orchestrator.spawn_bg_work().is_some());
    }
}
