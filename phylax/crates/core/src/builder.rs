use crate::{
    error::PhylaxNodeError,
    exit::PhylaxExitFuture,
    node::{PhylaxComponents, PhylaxNode},
    tasks::lifecycle::init_lifecycle_task,
    utils,
};
use futures::stream::{select_all, StreamExt};
use phylax_interfaces::{
    activity::{Activity, DynActivity},
    event::{EventBus, EventType},
    executors::{shutdown::GracefulShutdown, TaskExecutor},
    state_registry::StateRegistryBuilder,
};
use phylax_runner::{Backend, Compile, CoreBuildArgs, DataAccesses, ProjectPathsArgs};
use phylax_tasks::{
    activity::ActivityCell,
    task::{ActivityCategory, Task},
};
use phylax_tracing::{
    metrics::init_metrics,
    tracing::{debug, error, info, span, trace, warn, Instrument, Level, Span},
};
use std::{
    any::Any,
    path::PathBuf,
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
use tokio::{runtime::Handle, select, sync::oneshot, task::JoinHandle};

/// The [`PhylaxBuilder`] step in a Phylax node lifecycle.
///
/// This builder can be used to directly configure a default node,
/// as well as extend its functionality.
///
/// Returns a [`PhylaxNode`] which can be used to directly manage
/// a node.
#[derive(Default)]
pub struct PhylaxBuilder {
    /// All settings for how the node should be configured.
    //TODO: Use all config instances
    //pub config: PhylaxConfig,
    /// The optional tokio runtime handle.
    ///
    /// The tokio [`Handle`] is used to create the [`PhylaxNode`] task scheduler
    pub runtime_handle: Option<Handle>,
    /// The optional shutdown signal receiver that can be passed in for
    pub shutdown_receiver: Option<oneshot::Receiver<Option<Duration>>>,
    /// The optional custom Phylax activities.
    pub custom_activities: Vec<Box<dyn DynActivity>>,
    /// State Registry Builder, for registrying type demarcated shared state providers.
    pub state_registry_builder: StateRegistryBuilder,
    /// The path to the root of the project
    pub root: Option<PathBuf>,
}

impl PhylaxBuilder {
    /// Creates a new, empty Phylax node builder with default state providers.
    pub fn new() -> Self {
        let mut builder = Self::default();
        builder.state_registry_builder.insert(DataAccesses::default());
        builder.state_registry_builder.insert(Backend::spawn(None));
        builder
    }

    /// Sets the root directory of the project.
    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = Some(root);
        self
    }

    /// Sets the optional node Tokio runtime handle to be used.
    pub fn with_runtime_handle(mut self, runtime_handle: Handle) -> Self {
        self.runtime_handle = Some(runtime_handle);
        self
    }

    /// Sets custom activities that aren't defined by the config file, as well as the unique name of
    /// the concrete task they will turn into.
    pub fn with_custom_activity(
        mut self,
        task_name: impl Into<String>,
        activity: impl Activity + 'static,
    ) -> Self {
        let activity_object = Box::new(ActivityCell::new(task_name, activity));
        self.custom_activities.push(activity_object);
        self
    }

    /// Sets a state provider
    pub fn with_state_provider(
        mut self,
        provider: impl Any + Send + Sync,
    ) -> Option<Box<dyn Any + Send + Sync>> {
        self.state_registry_builder.insert(provider)
    }

    /// Launches the node and returns a handle to it.
    ///
    /// This bootstraps the node internals, sets up the
    /// core node execution task and lifecycle shutdown listener,
    /// compiles the smart contracts and returns
    /// a handle to await the node's shutdown.
    ///
    /// Returns a [`PhylaxNode`] that can be used to interact with the node.
    ///
    /// # Errors
    ///
    /// The only errors returned by the launch function are related to initializing
    /// a [`PhylaxNode`], not its execution. To track errors associated with the
    /// runtime of the node, await its [`PhylaxExitFuture`].
    pub async fn launch(mut self) -> Result<PhylaxNode, PhylaxNodeError> {
        // Initiate things such as the shutdown signals, exit future, and the task manager.
        let node_span = span!(parent: Span::none(), Level::INFO, "[Node]");
        init_metrics().map_err(PhylaxNodeError::TelemetryInitError)?;
        let event_bus = EventBus::new();
        let lifecycle_components = if let Some(runtime_handle) = self.runtime_handle {
            init_lifecycle_task(runtime_handle, event_bus.clone(), &node_span)
        } else {
            init_lifecycle_task(Handle::current(), event_bus.clone(), &node_span)
        };

        let (exit_tx, exit_rx) = oneshot::channel();

        let message_id_counter = Arc::new(AtomicU64::new(1));
        let task_executor = lifecycle_components.task_executor.clone();

        let timer = std::time::Instant::now();
        info!("Compiling Solidity files...");
        let evm_compilation_artifacts = CoreBuildArgs {
            project_paths: ProjectPathsArgs {
                root: self.root.clone(),
                contracts: self.root,
                ..Default::default()
            },
            ..Default::default()
        }
        .compile();
        info!(compile_time=?timer.elapsed(), "Finished Compiling Solidity files");

        if self.state_registry_builder.insert(evm_compilation_artifacts).is_some() {
            warn!("EvmCompilationArtifacts state provider in the state registry was overwritten by the builder.")
        }

        let tasks = utils::configure_tasks(
            self.custom_activities,
            task_executor,
            lifecycle_components.internal_shutdown_tx,
            event_bus.clone(),
            message_id_counter.clone(),
            &node_span,
            self.state_registry_builder.build(),
        )
        .await?;

        let task_names = tasks.iter().map(|task| task.details.task_name.clone()).collect();
        event_bus.send_unchecked(EventType::NodeStartup { task_names });

        lifecycle_components.task_executor.spawn_critical_with_graceful_shutdown_signal(
            "PhylaxNode",
            |shutdown| {
                Self::launch_node_task(tasks, shutdown, lifecycle_components.exit_handle, exit_tx)
                    .instrument(node_span.clone().or_current())
            },
        );

        let components = PhylaxComponents {
            task_executor: lifecycle_components.task_executor,
            node_span,
            message_id_counter,
            event_bus,
            //config: self.config,
        };

        let exit_future = PhylaxExitFuture::new(exit_rx);

        Ok(PhylaxNode {
            components,
            shutdown_signal: lifecycle_components.external_shutdown_tx,
            exit_future: Some(exit_future),
        })
    }

    /// The main task logic for the Phylax node, which executes all work needed to move through the
    /// entire Phylax Node lifecycle.
    ///
    /// When this function is called inside of a task, it will
    /// 1. Sets up all tasks initial state by calling their `bootstrap` method.
    /// 2. Sends all tasks into their main runtime using the `launch_tasks` method.
    /// 3. Awaits a shutdown signal from the `TaskManager` before cleaning up all tasks states by
    ///    calling their `closeout` method.
    /// 4. Sends the final shutdown result through the provided `exit_tx` channel.
    ///
    /// # Arguments
    ///
    /// * `tasks` - A vector of [`Arc`]-wrapped [`Task`]'s to be managed by the node.
    /// * `shutdown` - A [`GracefulShutdown`] signal to await on before initiating the shutdown
    ///   process.
    /// * `exit_handle` - A [`JoinHandle`] for the task managing the Node lifecycle, which returns a
    ///   result indicating success or failure.
    /// * `exit_tx` - A sender for signaling the final shutdown result.
    ///
    /// # Errors
    ///
    /// This function does not return errors, but will propagate any errors encountered during the
    /// shutdown process through the `exit_tx` channel to the [`PhylaxExitFuture`].
    async fn launch_node_task(
        mut tasks: Vec<Arc<Task>>,
        shutdown: GracefulShutdown,
        exit_handle: JoinHandle<Result<(), PhylaxNodeError>>,
        exit_tx: oneshot::Sender<Result<(), PhylaxNodeError>>,
    ) {
        debug!("Bootstrapping all tasks");
        for task in tasks.iter_mut() {
            task.bootstrap().await;
        }

        debug!("Launching all tasks");
        Self::launch_tasks(tasks);

        let shutdown_guard = shutdown.await;
        debug!("Phylax Node received a shutdown signal");

        let exit_result: Result<(), PhylaxNodeError> = match exit_handle.await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(shutdown_timeout_err)) => Err(shutdown_timeout_err),
            Err(join_err) => Err(join_err.into()),
        };

        debug!("Sending the final shutdown signal");
        let _ = exit_tx.send(exit_result);
        drop(shutdown_guard);
    }

    /// Launches the given tasks by spawning them using the task executor.
    ///
    /// This function iterates over the provided [`Task`] vector and spawns them based on their
    /// [`Activity`] type. Watcher tasks are spawned to poll their activities in a loop, while
    /// Action and Alert tasks are spawned to await new messages from their input receivers.
    ///
    /// # Arguments
    ///
    /// * `tasks` - A vector of [`Arc`]-wrapped [`Task`] to be launched.
    fn launch_tasks(tasks: Vec<Arc<Task>>) {
        let _ = tasks
            .into_iter()
            .map(|task| {
                let exec = task.task_executor.clone();
                let task_span = task.span.clone();
                match task.details.task_category {
                    ActivityCategory::Watcher => {
                        // If an activity is a watcher, create tasks that will simply poll the
                        // activity itself in a loop until there is more
                        // work to do
                        exec.spawn_critical_with_graceful_shutdown_signal(
                            "Watcher",
                            |shutdown_signal: GracefulShutdown| {
                                Self::run_watcher_task(task.clone(), shutdown_signal)
                                    .instrument(task_span.or_current())
                            },
                        )
                    }
                    ActivityCategory::Alert | ActivityCategory::Action => {
                        // If either Actions or Alerts, create tasks that await new messages from
                        // their message receivers
                        exec.spawn_critical_with_graceful_shutdown_signal(
                            "AlertAction",
                            |shutdown_signal| {
                                Self::run_alert_action_tasks(
                                    Arc::clone(&task),
                                    shutdown_signal,
                                    exec.clone(),
                                )
                                .instrument(task_span.or_current())
                            },
                        )
                    }
                }
            })
            .collect::<Vec<JoinHandle<()>>>();
    }

    /// Runs a watcher task that continuously loops to create new messages or handle the shutdown
    /// signal.
    ///
    /// This function operates in its own loop instead of polling a message queue, creating new
    /// messages until a shutdown signal is received. When the shutdown signal is detected, it
    /// performs cleanup activities and exits the loop.
    ///
    /// # Arguments
    ///
    /// * `task` - An [`Arc`]-wrapped [`Task`] that represents the watcher activity to be run.
    /// * `shutdown_signal` - A [`GracefulShutdown`] signal that indicates when the task should
    ///   perform cleanup and terminate.
    async fn run_watcher_task(task: Arc<Task>, mut shutdown_signal: GracefulShutdown) {
        loop {
            select! {
                // Process work and create new messages
                _ = task.process_message(None) => {
                    trace!("Creating a message");
                },
                // Shutdown signal detected, then run activity cleanup
                _ = &mut shutdown_signal => {
                    trace!("Closing out");
                    if let Err(err) = task.closeout().await {
                        error!(error = ?err, "There was a Task closeout error");
                    };
                    break;
                }
            }
        }
    }

    /// Runs alert or action tasks that await new messages from their message receivers, or handles
    /// the shutdown signal.
    ///
    /// This function operates by polling multiple message receivers and spawning new tasks to
    /// process incoming messages. When a shutdown signal is detected, it performs cleanup
    /// activities and exits the loop.
    ///
    /// # Arguments
    ///
    /// * `task` - An [`Arc`]-wrapped [`Task`] that represents the alert or action activity to be
    ///   run.
    /// * `shutdown_signal` - A [`GracefulShutdown`] signal that indicates when the task should
    ///   perform cleanup and terminate.
    /// * `exec` - A [`TaskExecutor`] that is used to spawn new tasks for processing messages.
    async fn run_alert_action_tasks(
        task: Arc<Task>,
        mut shutdown_signal: GracefulShutdown,
        exec: TaskExecutor,
    ) {
        let input_receivers = task.input_receivers.clone();
        let mut input_futs = select_all(
            input_receivers.iter().map(|receiver| receiver.stream()).collect::<Vec<_>>(),
        );
        let sub_span = span!(parent: task.span.clone(), Level::DEBUG, "[Subtask]");
        loop {
            let task_ref = Arc::clone(&task);
            select! {
                // Message found, therefore spawn new task to process the message
                input_msg = input_futs.next() => {
                    exec.spawn_critical("Task Processing", async move {
                        trace!("Processing a message");
                        task_ref.process_message(input_msg).await
                    }.instrument(sub_span.clone().or_current()));
                },
                // Shutdown signal detected, then run activity cleanup
                _ = &mut shutdown_signal => {
                    trace!("Closing out");
                    if let Err(err) = task_ref.closeout().await {
                        error!(error = ?err, "There was a Task closeout error");
                    };
                    break;
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_build_node_testdata() {
    PhylaxBuilder::new()
        .with_root(phylax_test_utils::ALERTS_BASE_PATH.into())
        .launch()
        .await
        .unwrap();
}
