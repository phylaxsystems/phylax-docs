use kanal::{AsyncReceiver, AsyncSender};
use opentelemetry::KeyValue;
use phylax_interfaces::{
    activity::{Label, PointValue},
    context::{ActivityContext, ActivityMonitor, HealthData, HealthMonitor},
    error::PhylaxError,
    event::{EventBus, EventType, TaskEventDetails},
    executors::TaskExecutor,
    state_registry::StateRegistry,
};
use phylax_tracing::tracing::{self, debug, error, instrument, span, trace, Level, Span};
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::{
    sync::{mpsc, Mutex},
    time::Instant,
};

use crate::{
    metrics::TaskMetrics,
    monitors::{build_monitor_channels, MonitorChannels, MonitorReceiver},
};
use phylax_interfaces::{activity::DynActivity, error::PhylaxTaskError, message::BoxedMessage};

/// Phylax task state and lifecycle manager
///
/// The Task wraps the type-erased [`DynActivity`] trait object, which allows it to safely
/// manage the activity lifecycle and contexts relevant to the task without the task runtime
/// in the node needing to be concerned about the specific contexts or underlying associated types.
pub struct Task {
    /// The details of the task, containing the task name, type, and category. Grouped here for
    /// easy reuse for metrics and tracing.
    pub details: TaskDetails,
    /// The parent task executor for spawning unrecoverable tasks.
    pub task_executor: TaskExecutor,
    /// The input message receivers for this task, to be the source of messages consumed by the
    /// inner activity message processing function.
    pub input_receivers: Vec<AsyncReceiver<BoxedMessage>>,
    /// The output message senders for this task, to be the channel which messages produced by the
    /// inner activity message processing function are sent over.
    pub output_sender: Option<AsyncSender<BoxedMessage>>,
    /// The atomic unsigned integer which represents the next unique id of the message to be sent
    /// by the activity.
    pub message_counter: Arc<AtomicU64>,
    /// The shutdown sender for signalling to the node runtime to halt.
    pub shutdown_signal: mpsc::UnboundedSender<Result<(), PhylaxTaskError>>,
    /// The tracing span associated with the task.
    pub span: Span,
    /// The global metrics provider:
    pub metrics: TaskMetrics,
    /// The global event bus used for sending non-processing related messages across the system.
    pub event_bus: EventBus,
    /// Senders for default activity monitor channels for this task.
    activity_senders: HashMap<&'static str, ActivityMonitor>,
    /// Senders for health monitor channels for this task.
    health_senders: HashMap<&'static str, HealthMonitor>,
    /// The underlying activity trait object.
    pub activity: Mutex<Box<dyn DynActivity>>,
    /// Type Demarcated Shared Global state
    pub state_registry: Arc<StateRegistry>,
}

/// Struct representing the details of a task, including its name, type, and category.
/// This information is used for metrics and tracing purposes.
#[derive(Debug, Clone)]
pub struct TaskDetails {
    /// The unique task name, representing a concrete task or activity, which is distinct from the
    /// const name of the type of the inner activity.
    pub task_name: String,
    /// The type of the inner activity, which is the concrete type of the activity that implements
    /// the [`Activity`] trait.
    pub task_flavor: String,
    /// The category of the activity, which vary for an Activity depending on whether it listens
    /// for an input type, an output type, or both.
    pub task_category: ActivityCategory,
    /// The labels associated with the task, which are used for display purposes.
    pub task_labels: &'static [Label],
}

/// Enum that represents the possible types of activities that a task might manage.
///
/// Internally, there are 3 categories of activities that can be represented: Watchers, Alerts, and
/// Actions. These categories represent cases of vertices or node on a directed graph of
/// activities, where Watchers are nodes with edges or connections directed at to other
/// activities but which no edges are directed at it, Alerts are nodes that have connections either
/// directed at it or directing from it, and Actions are terminal nodes with only connections
/// directed at it.
#[derive(Debug, Clone)]
pub enum ActivityCategory {
    /// Watchers represent an activity that only produce messages, never consuming them.
    Watcher,
    /// Alerts represent an activity that consumes and produces messages.
    Alert,
    /// Actions represent an activity that only consumes messages.
    Action,
}

pub struct TaskChannels {
    pub maybe_outbox_receiver: Option<AsyncReceiver<BoxedMessage>>,
    pub activity_receivers: Vec<MonitorReceiver<PointValue>>,
    pub health_receivers: Vec<MonitorReceiver<HealthData>>,
}

impl Task {
    /// Creates a task but does not bootstrap the inner activity.
    pub fn init(
        executor: TaskExecutor,
        event_bus: EventBus,
        message_counter: Arc<AtomicU64>,
        activity: Box<dyn DynActivity>,
        shutdown_signal: mpsc::UnboundedSender<Result<(), PhylaxTaskError>>,
        node_span: &Span,
        state_registry: Arc<StateRegistry>,
    ) -> (Task, TaskChannels, ComponentDisplayInfo) {
        // The unit type [`()`] represents a special condition of an "empty type" when marked as the
        // input or output in an Activity definition that denotes special behavior for the
        // Activity functioning as a Watcher or Action rather than an Action.
        let empty_type_id = TypeId::of::<()>();

        let activity_category = match (
            activity.get_input_type_id() == empty_type_id,
            activity.get_output_type_id() == empty_type_id,
        ) {
            // Alert is Input = SomeType, Output = SomeType
            (false, false) => ActivityCategory::Alert,
            // Watcher is Input = (), Output = SomeType
            (true, false) => ActivityCategory::Watcher,
            // Action is Input = SomeType, Output = ()
            (false, true) => ActivityCategory::Action,
            // Input = (), Output = (), this is invalid
            (true, true) => unimplemented!(),
        };

        let (maybe_output_sender, maybe_output_receiver) =
            if let ActivityCategory::Action = activity_category {
                (None, None)
            } else {
                let (sender, receiver) = kanal::unbounded_async();
                (Some(sender), Some(receiver))
            };

        let details = TaskDetails {
            task_name: activity.get_activity_name(),
            task_flavor: activity.get_activity_flavor().to_string(),
            task_category: activity_category,
            task_labels: activity.get_labels(),
        };

        let metrics = TaskMetrics::new(details.clone());
        let span = span!(
            parent: node_span,
            Level::INFO, "[Task]", 
            task_name=details.task_name,
            task_flavor=details.task_flavor,
            task_category=?details.task_category);

        let MonitorChannels {
            activity_senders,
            activity_receivers,
            health_senders,
            health_receivers,
        } = build_monitor_channels(activity.get_monitor_configs(), &details, &metrics.otel_context);

        let task_channels = TaskChannels {
            maybe_outbox_receiver: maybe_output_receiver,
            activity_receivers,
            health_receivers,
        };

        let task = Task {
            details,
            task_executor: executor,
            event_bus,
            shutdown_signal,
            input_receivers: Vec::new(),
            output_sender: maybe_output_sender,
            span,
            metrics,
            activity_senders,
            health_senders,
            activity: Mutex::new(activity),
            message_counter,
            state_registry,
        };

        let component_display_info = ComponentDisplayInfo::from(&task);

        trace!("Initialized task");

        (task, task_channels, component_display_info)
    }

    /// Bootstrap the inner activity after the task has been initialized.
    #[instrument(parent = &self.span, skip(self))]
    pub async fn bootstrap(&self) {
        let context = self.get_context();
        let mut guard = self.activity.lock().await;
        trace!("Task attempting to bootstrap the activity");
        if let Err(err) = guard.bootstrap(context).await {
            error!(error = ?err, "Failed to bootstrap");
            // Ignore returned error because it indicates the shutdown signal was already received
        } else {
            trace!("Successfully bootstrapped");
        }
    }

    /// The process message function, which runs the underlying function in the inner activity
    /// and then potentially panics to prematurely end the task prematurely based on the
    /// severity of the error returned.
    #[instrument(parent = &self.span, skip(self, input))]
    pub async fn process_message(&self, input: Option<BoxedMessage>) {
        let context = self.get_context();
        let start_time = Instant::now();
        let processed_result = self
            .activity
            .lock()
            .await
            .process_message(input, context, self.message_counter.clone())
            .await;
        let process_duration = start_time.elapsed().as_micros() as u64;
        let result_context = self.handle_processed_result(processed_result).await;
        self.metrics.record_processing(process_duration, &result_context);
    }

    async fn handle_processed_result(
        &self,
        processed_result: Result<Option<BoxedMessage>, PhylaxTaskError>,
    ) -> Vec<KeyValue> {
        match processed_result {
            Ok(Some(message)) => {
                // A message was successfully created after processing, and is now ready to send.
                let message_id = message.details.id;
                trace!(message_id, "Task successfully created an output message, but did not have an output channel configured for sending it");
                if let Some(sender) = &self.output_sender {
                    // During normal execution, there should never be instances where messages are
                    // sent to a dropped or closed receiver except during shutdown
                    if let Err(err) = sender.send(message).await {
                        error!(error = ?err, message_id, "Encountered a critical error after failing to send an message over a channel");
                        // Ignore returned error because it indicates the shutdown signal was
                        // already received
                        let _ = self.shutdown_signal.send(Err(PhylaxTaskError::from(err)));
                        vec![KeyValue::new("is_error", true), KeyValue::new("is_critical", true)]
                    } else {
                        trace!(message_id, "Task successfully created and sent an output message");
                        let _ = self.event_bus.send(EventType::TaskSentMessage(self.into()));
                        vec![KeyValue::new("is_error", false), KeyValue::new("has_message", true)]
                    }
                } else {
                    trace!(message_id, "Task successfully created an output message, but did not have an output channel configured for sending it");
                    vec![KeyValue::new("is_error", false), KeyValue::new("has_message", false)]
                }
            }
            Ok(None) => {
                // An output message was not created, which is by usually the message processing
                // behavior of Actions, or was intended by the Activity to run as a
                // no-op.
                trace!("Task successfully processed work without an output message");

                self.event_bus.send_unchecked(EventType::TaskSentNoMessage(self.into()));
                vec![KeyValue::new("is_error", false), KeyValue::new("has_message", false)]
            }
            Err(PhylaxTaskError::ActivityProcessError(PhylaxError::RecoverableError(err))) => {
                // An Activity-defined, recoverable activity error which allows the task to continue
                // running as normal.
                error!(error = ?err, "An activity encountered a recoverable error");
                let details: TaskEventDetails = self.into();
                self.event_bus.send_unchecked(EventType::TaskRecoverableUserError(
                    details.with_error_msg(err.to_string()),
                ));
                vec![KeyValue::new("is_error", true), KeyValue::new("is_critical", false)]
            }
            Err(PhylaxTaskError::ActivityProcessError(PhylaxError::UnrecoverableError(err))) => {
                // An Activity-defined critical error which causes a node shutdown.
                error!(error = ?err, "An activity encountered an unrecoverable error");
                let details: TaskEventDetails = self.into();
                self.event_bus.send_unchecked(EventType::TaskRecoverableUserError(
                    details.with_error_msg(err.to_string()),
                ));
                vec![KeyValue::new("is_error", true), KeyValue::new("is_critical", true)]
            }
            Err(err) => {
                // General unexpected system or task errors such as errors from being unable
                // to decorate messages from system time being unavailable.
                error!(error = ?err, "A task encountered an unrecoverable system error");

                let details: TaskEventDetails = self.into();
                self.event_bus.send_unchecked(EventType::TaskSystemError(
                    details.with_error_msg(err.to_string()),
                ));
                // Ignore returned error because it indicates the shutdown signal was already
                // received.
                let _ = self.shutdown_signal.send(Err(err));
                vec![KeyValue::new("is_error", true), KeyValue::new("is_critical", true)]
            }
        }
    }

    /// The closeout function which runs the activity cleanup method and is triggered by a task
    /// runner level shutdown signal.
    #[instrument(parent = &self.span, skip(self))]
    pub async fn closeout(&self) -> Result<(), PhylaxTaskError> {
        let context = self.get_context();
        debug!("Atempting to shutdown and cleanup the activity");
        self.activity.lock().await.cleanup(context).await
    }

    /// Builds the relevant context for the underlying activity functions.
    fn get_context(&self) -> ActivityContext {
        ActivityContext {
            executor: self.task_executor.clone(),
            event_bus: self.event_bus.clone(),
            activity_monitors: self.activity_senders.clone(),
            health_monitors: self.health_senders.clone(),
            state_registry: self.state_registry.clone(),
        }
    }
}

pub struct ComponentDisplayInfo {
    pub name: String,
    pub flavor: String,
    pub description: Option<String>,
    pub category: ComponentDisplayCategories,
    pub labels: HashMap<String, String>,
}

pub enum ComponentDisplayCategories {
    Watcher,
    Alert,
    Action,
    Notification,
}

impl From<&Task> for TaskEventDetails {
    fn from(val: &Task) -> Self {
        TaskEventDetails {
            task_name: val.details.task_name.clone(),
            task_type: val.details.task_flavor.clone(),
            error_msg: None,
        }
    }
}

impl From<&ActivityCategory> for ComponentDisplayCategories {
    fn from(val: &ActivityCategory) -> Self {
        match val {
            ActivityCategory::Watcher => ComponentDisplayCategories::Watcher,
            ActivityCategory::Alert => ComponentDisplayCategories::Alert,
            ActivityCategory::Action => ComponentDisplayCategories::Action,
        }
    }
}

impl From<&Task> for ComponentDisplayInfo {
    fn from(val: &Task) -> Self {
        let mut labels = HashMap::new();
        for Label { key, value } in val.details.task_labels.iter() {
            labels.insert(key.to_string(), value.to_string());
        }
        ComponentDisplayInfo {
            name: val.details.task_name.clone(),
            flavor: val.details.task_flavor.clone(),
            description: None,
            category: (&val.details.task_category).into(),
            labels,
        }
    }
}
