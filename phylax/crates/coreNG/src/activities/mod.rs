use crate::{
    events::{Event, EventBus},
    tasks::{dns::TaskDns, task::WorkContext},
};
use async_trait::async_trait;
use futures_util::Stream;
use phylax_common::metrics::MetricsRegistry;
use std::{collections::HashMap, fmt::Display, sync::Arc};
use thiserror::Error;
use tokio::{sync::broadcast, task::JoinHandle};

pub mod system;
pub mod user_defined;

/// The Activity trait defines the behavior of an activity in the system.
/// Activities are units of work that can be scheduled and executed.
/// They can be either system-defined or user-defined.
#[async_trait]
pub trait Activity: Send + Sync + Display
where
    Self: Sized,
{
    /// The type of command that this activity can process.
    type Command: Send + Sync + std::fmt::Debug;
    /// The type of internal stream that this activity uses.
    type InternalStream: Stream<Item = Self::InternalStreamItem> + Send + Unpin;
    /// The type of item that the internal stream produces.
    type InternalStreamItem: Send + std::fmt::Debug;

    /// The bootstrap method is called when the activity is initialized.
    /// It can be used to set up any necessary state or resources.
    async fn bootstrap(&mut self, _context: Arc<ActivityContext>) -> eyre::Result<()> {
        Ok(())
    }

    /// The decide method is called to determine what commands should be issued based on the current
    /// events and context.
    fn decide(&self, _events: Vec<&Event>, _context: &WorkContext) -> Option<Vec<Self::Command>> {
        None
    }

    /// The do_work method is called to process a command.
    /// It is expected to be non-blocking and yield to the async runtime quickly.
    async fn do_work(
        &mut self,
        _command: Vec<Self::Command>,
        _context: WorkContext,
    ) -> eyre::Result<()> {
        Err(ActivityError::NotImplemented.into())
    }

    /// The do_work_blocking method is called to process a command.
    /// It is expected to be blocking and may take a long time to return.
    fn do_work_blocking(
        &mut self,
        _command: Vec<Self::Command>,
        _context: WorkContext,
    ) -> eyre::Result<()> {
        Err(ActivityError::NotImplemented.into())
    }

    /// The work_type method is used to determine the type of work that this activity performs.
    /// Depending on the [`FgWorkType`](FgWorkType) that is returned, the task will either invoke
    /// `do_work` or `do_work_blocking`.
    fn work_type(&self) -> FgWorkType {
        Default::default()
    }

    /// The cleanup method is called when the activity is finished. More specifically, it will
    /// called when the task is being shutdown or it encounters an unrecoverable error or it
    /// panics. It can be used to clean up any resources or state.
    async fn cleanup(self) -> eyre::Result<()> {
        Err(ActivityError::NotImplemented.into())
    }

    /// The internal_stream method is used to get a stream of internal events that this activity
    /// produces. The Internal stream enables the Task to react to events that are task-specific and
    /// entirely irrelevant to the event bus. For example, it can be new blocks on the
    /// blockchain or the result of some computation.
    /// This function returns the internal stream so that the Task can `await` it in the main loop.
    /// If it returnes [`None`](Option::None), then the Activity doesn't use any internal stream.
    /// It's just a `no-op`.
    async fn internal_stream(&self) -> Option<Self::InternalStream> {
        None
    }

    /// The internal_stream_work method is called to process an item from the internal stream.
    /// Whenever a new item is produced by the stream returned by
    /// [`internal_stream`](Activity::internal_stream), this function will be called to process the
    /// event.
    async fn internal_stream_work(
        &mut self,
        _item: Self::InternalStreamItem,
        _ctx: Arc<ActivityContext>,
    ) -> Option<Event> {
        None
    }

    /// The spawn_bg_work method is used to spawn a background task for this activity. This is
    /// usually used in conjuction with [`Activity::InternalStream`]. We return
    /// the tokio task so that we can catch any errors or panics. If the Activity doesn't
    /// spawn any background task, then simply return [`None`](Option::None).
    fn spawn_bg_work(&mut self) -> Option<JoinHandle<eyre::Result<()>>> {
        None
    }
}

/// The ActivityContext struct is used to provide context to the activities.
/// It contains the task name, metrics registry, dns, and event bus.
/// By using the context, the activity has access to:
/// - The name of the task
/// - The names and IDs of all the tasks.
/// - The metrics Registry, so that it can expose any useful metrics via the Prometheus endpoint
/// - The event bus, so that it can send events to the event bus
/// - The internal signal to shutdown the task of the activity. Useful to programmatically shutdown
///   the task.
#[derive(Debug)]
pub struct ActivityContext {
    /// The name of the task.
    pub task_name: String,
    /// The metrics registry.
    pub metrics: MetricsRegistry,
    /// The DNS.
    pub dns: Arc<TaskDns>,
    /// The event bus.
    pub bus: EventBus,
    /// signal to shutdown the task of the activity
    pub shutdown: broadcast::Sender<()>,
}

impl ActivityContext {
    /// Creates a new ActivityContext.
    ///
    /// # Arguments
    ///
    /// * `task_name` - The name of the task.
    /// * `metrics_registry` - The metrics registry.
    /// * `dns` - The DNS.
    /// * `bus` - The event bus.
    pub fn new(
        task_name: String,
        metrics_registry: MetricsRegistry,
        dns: Arc<TaskDns>,
        bus: EventBus,
        shutdown: broadcast::Sender<()>,
    ) -> Self {
        Self { task_name, metrics: metrics_registry, dns, bus, shutdown }
    }

    pub fn get_erased_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("activity.task_name".to_string(), self.task_name.clone());
        map
    }

    pub fn get_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert(
            "activity.task_name".to_string(),
            serde_json::Value::String(self.task_name.clone()),
        );
        map
    }
}

/// The FgWorkType enum is used to specify the type of foreground work that an activity can perform.
/// It can be one of the following:
/// - None: The activity does not perform any foreground work.
/// - NonBlocking: The activity performs non-blocking foreground work. [`Activity::do_work`] will be
///   invoked in main loop of the Task. This is helpful if the Task wants to `do_work` on every
///   event,
/// but if `do_work` takes too long, it can block the Task's ability to process events and
/// eventually lag behind the event bus.
/// - SpawnBlocking: The activity spawns a new task and executes the non-async work.
/// - SpawnNonBlocking: The activity spawns a new task for non-blocking foreground work.
/// Read more about Task's lifecycle in [`super::tasks::task::Task`]
/// ```
/// let work_type = FgWorkType::NonBlocking;
/// ```
#[derive(Default, PartialEq, Debug)]
pub enum FgWorkType {
    /// The default value. It means the activity does not perform any foreground work.
    #[default]
    None,
    /// The activity performs non-blocking foreground work.
    NonBlocking,
    /// The activity spawns a new task for blocking foreground work.
    SpawnBlocking,
    /// The activity spawns a new task for non-blocking foreground work.
    SpawnNonBlocking,
}

#[derive(Debug, Error)]
#[error("Activity error")]
pub enum ActivityError {
    #[error("The type of work is not implemented. This is a bug.")]
    NotImplemented,
    #[error("The type of work is not implemented. This is a bug.")]
    BespokeError(Box<dyn std::error::Error + Send + Sync>),
    #[error("The command '{0}' has no 'work' implemented")]
    UnknownCommand(String),
    #[error("Activity errored at bootstrap")]
    BootstrapError,
}

impl<T> From<flume::SendError<T>> for ActivityError
where
    T: Send + Sync + 'static,
{
    fn from(error: flume::SendError<T>) -> Self {
        ActivityError::BespokeError(Box::new(error))
    }
}
