use async_trait::async_trait;
use eyre::Result;
use futures_util::{future, Future, StreamExt};
use phylax_common::metrics::MetricsRegistry;
use phylax_tracing::tracing::{
    debug, debug_span, info, info_span, instrument, trace, warn, Instrument, Span,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sisyphus_tasks::{
    sisyphus::{ErrExt, ShutdownSignal},
    Boulder, Fall,
};
use std::{
    collections::HashMap,
    fmt::{self, Debug, Display},
    pin::Pin,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
    time::Duration,
};
use tokio::{
    pin,
    sync::{broadcast, Mutex},
    task::JoinHandle,
};

use crate::{
    activities::{Activity, ActivityContext, FgWorkType},
    events::{Event, EventBuilder, EventBus, EventType},
    tasks::dns::TaskDns,
};

use super::{error::TaskError, event_buffer::EventBuffer, subscriptions::Subscriptions};

/// The `Task` struct represents a task in the system.
/// It contains all the necessary information and state for a task to function.
/// The `Task` struct is parameterized over `T` where `T` is an `Activity`.
pub struct Task<T>
where
    T: Activity + 'static,
{
    /// The name of the task.
    pub name: String,
    /// The unique identifier of the task.
    pub id: u32,
    /// The span for tracing the task.
    pub span: Span,
    /// The metrics registry for the task.
    pub metrics: MetricsRegistry,
    /// The category of the task.
    pub category: TaskCategory,
    /// The event bus for the task.
    pub event_bus: EventBus,
    /// The DNS for the task.
    pub dns: Arc<TaskDns>,
    /// The subscriptions of the task.
    pub subscriptions: Subscriptions,
    /// The event buffer for the task.
    pub buffer: EventBuffer,
    /// The inner activity of the task.
    pub inner: Arc<Mutex<T>>,
    /// A flag indicating whether the task is working in the foreground.
    pub fg_working: AtomicBool,
    /// A flag indicating whether the task is working in the background.
    pub bg_working: AtomicBool,
    pub internal_shutdown_tx: broadcast::Sender<()>,
    /// The handle for the background task, if it exists.
    pub maybe_bg_handle: Option<JoinHandle<eyre::Result<()>>>,
    /// The handle for the background task, if it exists.
    pub maybe_fg_handle: Option<JoinHandle<eyre::Result<()>>>,
}

#[derive(PartialEq, Eq, Default, Debug)]
pub enum TaskCategory {
    #[default]
    UserDefined,
    System,
}

impl FromStr for TaskCategory {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "userdefined" => Ok(TaskCategory::UserDefined),
            "system" => Ok(TaskCategory::System),
            _ => Err(eyre::Report::msg(format!("Unknown Task Category: {s}"))),
        }
    }
}

#[async_trait]
/// The `PhylaxTask` trait defines the common behavior for all tasks in the Phylax system.
/// It includes methods for bootstrapping the task, reading from the event bus, sending events,
/// handling events, checking subscription status, storing events to buffer, and getting the task's
/// name and id.
pub trait PhylaxTask: Display + Boulder {
    /// Bootstraps the task with the given activity context.
    /// Returns a `Result` indicating the success or failure of the operation.
    async fn bootstrap(&mut self, ctx: Arc<ActivityContext>) -> Result<(), TaskError>;

    /// Reads an event from the task's event bus.
    /// Returns a `Result` containing the event or an error if the operation fails.
    async fn read_event_bus(&mut self) -> Result<Event, TaskError>;

    /// Sends an event to the task's event bus.
    /// The event is converted from the generic type `E` into an `Event`.
    /// Returns a `Result` indicating the success or failure of the operation.
    fn send_event<E: Into<Event>>(&mut self, event: E) -> Result<(), TaskError>;

    /// Handles an incoming event with the given activity context.
    /// Returns a `Result` containing an optional [`tokio::task::JoinHandle`] for the task handling
    /// the event, or an error if the operation fails.
    async fn handle_event(
        &mut self,
        event: Event,
        ctx: WorkContext,
    ) -> Result<Option<JoinHandle<eyre::Result<()>>>, TaskError>;

    /// Checks if the task is subscribed to and ready for the given event.
    /// Returns a `Result` containing a boolean indicating the subscription status,
    /// or an error if the operation fails.
    fn is_subscribed_and_ready(&mut self, event: &Event) -> Result<bool, TaskError>;

    /// Stores the given event to the task's event buffer.
    /// Returns an `Option` containing the event if it was successfully stored, or `None` otherwise.
    fn store_event_to_buffer(&mut self, event: Event) -> Option<Event>;

    /// Returns the name of the task.
    fn name(&self) -> &str;

    /// Returns the unique identifier of the task.
    fn id(&self) -> u32;
}

/// This function is responsible for bootstrapping a task.
/// It prepares the task for execution by storing its name in the DNS and bootstrapping its inner
/// activity. If the bootstrap process fails, it returns a `TaskError::BootstrapError`.
/// If it succeeds, it logs a message and returns [`core::result::Result::Ok(())`]
///
/// # Arguments
///
/// * `&mut self` - A mutable reference to the instance of the task.
/// * `context: &ActivityContext` - A reference to the activity context.
///
/// # Returns
///
/// * `Result<(), TaskError>` - Returns [`core::result::Result::Ok(())`] if the bootstrap process is
///   successful, otherwise returns [`TaskError::BootstrapError`].
#[async_trait]
impl<T> PhylaxTask for Task<T>
where
    T: Activity + 'static,
{
    #[instrument(skip(self, context), level = "info")]
    async fn bootstrap(&mut self, context: Arc<ActivityContext>) -> Result<(), TaskError> {
        // Lock the inner activity of the task
        let mut activity = self.inner.lock().await;
        // Store the task name in the DNS
        let _ = self.dns.store_name(&self.name);
        // Bootstrap the inner activity of the task
        activity.bootstrap(context).await.map_err(|e| TaskError::BootstrapError { source: e })?;
        // Log a message indicating that the bootstrap process is successful
        info!("Bootstraped");
        // Return Ok(())
        Ok(())
    }

    // Handles an incoming event.
    ///
    /// This function takes an event as input and processes it according to the task's logic.
    /// It returns a future representing the ongoing work, or an error if the event could not be
    /// handled.
    #[instrument(skip(self, context, event), level = "info")]
    async fn handle_event(
        &mut self,
        event: Event,
        context: WorkContext,
    ) -> Result<Option<JoinHandle<eyre::Result<()>>>, TaskError> {
        // Log the incoming event at trace level
        trace!(?event, "Handling event");

        // Check if the event is subscribed and ready for processing
        if !self.is_subscribed_and_ready(&event)? {
            // If not, return None immediately
            return Ok(None);
        }

        // Log that the event is subscribed and ready
        trace!("Subscribed and ready");

        // Store the event into a buffer for later processing
        let _kicked_event = self.store_event_to_buffer(event);

        // Log that the event has been stored to buffer
        trace!("Stored event to buffer");

        // Check if there's any ongoing foreground work
        if self.fg_working.load(Relaxed) {
            // If there is, log the situation and return None immediately
            trace!("Task has an ongoing foreground work; It will process the new event later");
            return Ok(None);
        }

        // Lock the inner activity and decide the next command to execute
        let activity = self.inner.lock().await;
        let command_or_none = activity.decide((&self.buffer).into(), &context);

        // Clear the buffer and release the lock on the inner activity
        self.buffer.clear();
        drop(activity);

        // If a command is decided, execute the command and return the result
        if let Some(cmd) = command_or_none {
            // Log the decided command
            trace!(command=?cmd,"Decided command");
            self.do_work(cmd, context).await
        } else {
            // If no command is decided, return None
            Ok(None)
        }
    }

    /// Returns the id of the task.
    fn id(&self) -> u32 {
        self.id
    }

    /// Checks if the task is subscribed and ready for the given event.
    /// Returns a boolean indicating the readiness of the task.
    /// Returns an error of type [`TaskError`] if the check fails.
    fn is_subscribed_and_ready(&mut self, event: &Event) -> Result<bool, TaskError> {
        let span = debug_span!("is_subscribed_and_ready");
        let _guard = span.enter();
        let res = self.subscriptions.is_subscribed_and_ready(event)?;
        Ok(res)
    }

    /// Returns the name of the task.
    fn name(&self) -> &str {
        &self.name
    }

    /// Reads an event from the event bus.
    /// Returns the event if successful, or an error of type [`TaskError`] if the read fails.
    async fn read_event_bus(&mut self) -> Result<Event, TaskError> {
        self.event_bus.read().await.map_err(|e| e.into())
    }

    /// Sends an event to the event bus.
    /// Returns an error of type [`TaskError`] if the send fails.
    fn send_event<E>(&mut self, into_event: E) -> Result<(), TaskError>
    where
        E: Into<Event>,
    {
        let mut event = into_event.into();
        event.set_origin(self.id);
        Ok(self.event_bus.publish(event)?)
    }

    /// Stores an event to the buffer.
    /// If the buffer is full, pops an event from the buffer before pushing the new event.
    /// Returns the popped event if the buffer was full, or None otherwise.
    fn store_event_to_buffer(&mut self, event: Event) -> Option<Event> {
        let kicked_event = if self.buffer.is_full() { self.buffer.pop() } else { None };
        // can't fail because we pop an element if it's full, before pushing
        let _ = self.buffer.push(event);
        kicked_event
    }
}

impl<T> Boulder for Task<T>
where
    T: Activity + 'static,
    Task<T>: PhylaxTask,
{
    /// Spawns a new task.
    ///
    /// This function takes ownership of `self` and a `shutdown` signal, and returns a
    /// [`tokio::task::JoinHandle`]. The returned [`JoinHandle`] can be used to await the task.
    ///
    /// The function performs the following steps:
    /// 1. It bootstraps the task. If an error occurs during bootstrapping, it returns an
    ///    [`sisyphus::Fall::Unrecoverable`].
    /// 2. It initializes `maybe_fg_handle`, `temp`, `maybe_stream`, and `maybe_bg_handle`.
    /// 3. It enters a loop where it does the following:
    ///     - It enters a [`tokio::select`] block where it does one of the following:
    ///         - If the `shutdown` signal is received, it logs a message and returns a
    ///           [`sisyphus::Fall::Shutdown`] error.
    ///         - If an internal event is received, it processes the event and sends it to the event
    ///           bus. If an error occurs while sending the event, it returns an
    ///           [`sisyphus::Fall::Unrecoverable`]
    ///         - If a background task finishes, it processes the result of the task.
    /// The function continues to loop until the [`ShutdownSignal`] signal is received.
    fn spawn(mut self, mut shutdown: ShutdownSignal) -> tokio::task::JoinHandle<Fall<Self>>
    where
        Self: Send,
    {
        let span = info_span!(parent: &self.span, "main_loop");
        tokio::task::spawn(async move {

            // Get the context for the activity. The context is useful information
            // or data for the activity, such as the name of the task.
            let context = self.get_context();
            debug!("Spawning Activity with context");

            // Set task to working while bootstraping
            let _ = self.set_working_fg(true);
            if let Err(err) = self.bootstrap(context.clone()).await {
                return err.unrecoverable(self, true);
            }


            // Bootstrap is finished, set to idle
            let _ = self.set_working_fg(false);

            // Get a lock to the activity
            let mut temp = self.inner.lock().await;

            // Try to get the background stream
            let mut maybe_stream = temp.internal_stream().await;
            if maybe_stream.is_some() {
                debug!("Task has an internal stream");
            }
            self.maybe_bg_handle = if let Some(handle) = temp.spawn_bg_work() {
                debug!("[Background Work] spawned");
                // We don't need the activity any longer
                drop(temp);
                let _res = self.set_working_bg(true);
                Some(handle)
            } else {
                drop(temp);
                None
            };
            let mut internal_rx= self.internal_shutdown_tx.subscribe();
            let internal_shutdown =internal_rx.recv();
            pin!(internal_shutdown);
            let shutdowns = future::select(internal_shutdown, &mut shutdown);
            pin!(shutdowns);
            // Enter the main loop
            loop {
                tokio::select! {
                        biased;
                        _ = &mut shutdowns => {
                            debug!("[Task] Shutdown signal");
                            return Fall::Shutdown { task: self }
                        },
                        // If there is an internal stream, check for new items
                        Some(maybe_internal_event) = async {
                            if let Some(ref mut stream) = maybe_stream {
                                pin!(stream);
                                Some(stream.next().await)
                            } else {
                                None
                            }
                        } => {
                            if let Some(internal_event) = maybe_internal_event {
                                trace!(?internal_event, "[Internal Event]");
                                // Process the internal item
                                let maybe_event= self.inner.lock().await.internal_stream_work(internal_event, context.clone()).await;
                                // If there is an event, as a result of the internal item process: emit it!
                                if let Some(event) = maybe_event{
                                    trace!(%event, "[Event] Outgoing");
                                    if let Err(err) = self.send_event(event) {
                                        return err.unrecoverable(self, true)
                                    }
                                }
                            } else {
                                // this is not an error, it simply signifies that no new internal events are available
                                continue
                            }
                        },
                        // if there is some work in the background going,
                        // lock the tokio task and await. This is used to catch
                        // any errors in the backgrond task or to know when
                        // the background work finished.
                        Some(handle) = async {
                            if let Some(bg_handle) = self.maybe_bg_handle.as_mut() {
                                Some(bg_handle.await)
                            } else {
                                None
                            }
                        } => {
                            let error = match handle {
                                Ok(res) => {
                                    if let Err(bg_error) = res {
                                        warn!(error=%bg_error,"[Background Work] Error");
                                       TaskError::BgWorkError { source: bg_error }
                                    } else {
                                        debug!("[Background Work] Finished");
                                        let _ = self.set_working_bg(false);
                                        continue
                                    }
                                },
                                Err(_err) => {
                                    TaskError::BgHandleError
                                },
                            };
                            return error.recoverable(self, shutdown)
                        },
                        // If there is foreground work going, try to await it
                        Some(handle) = async {
                            if let Some(fg_handle) = self.maybe_fg_handle.as_mut(){
                                Some(fg_handle.await)
                            } else {
                                None
                            }
                        } => {
                            let error = match handle {
                                Ok(res) => {
                                    if let Err(fg_error) = res {
                                        warn!(error=%fg_error,"[Foreground Work] Error");
                                        TaskError::FgWorkError { source: fg_error}
                                    } else {
                                        debug!("[Foreground Work] Finished");
                                        let _res = self.set_working_fg(false);
                                        self.maybe_fg_handle = None;
                                        continue
                                    }
                                },
                                Err(_err) => {
                                    TaskError::FgHandleError
                                },
                            };
                            return error.recoverable(self, shutdown)
                        },
                        // If there is a new event in the event_bus, read and handle it
                         event_or_error = self.event_bus.read() => {
                            match event_or_error {
                                Ok(event) => {
                                    debug!(?event, buffer_free_percentage=self.buffer.free_percentage(), bus_free=self.event_bus.free_percentage(), "[Event] Ingoing");
                                    let work_context = WorkContext::new(self.get_context(), EventContext::new(event.clone()));
                                    match self.handle_event(event, work_context).await {
                                        // If the handle_event returns a tokio task handle to 
                                        // foreground work. Add it to variable so that we can
                                        // await it in the next iteration of the loop
                                        Ok(handle_or_none) => {
                                            if handle_or_none.is_some(){
                                                debug!("[Foreground Work] Spawned");
                                            }
                                            self.maybe_fg_handle = handle_or_none;
                                        },
                                        Err(e) => return e.recoverable(self, shutdown)
                                    }
                                },
                                Err(e) => {
                                    warn!("Event bus encountered error: {e}")
                                }
                            }
                        },

                }
            }
        }.instrument(span))
    }

    /// This function is responsible for cleaning up a task.
    /// It aborts any ongoing background work, cleans up the inner activity, and logs a message.
    /// If the cleanup process fails, it returns an error.
    ///
    /// # Arguments
    ///
    /// * `self` - The instance of the task.
    ///
    /// # Returns
    ///
    /// * `Pin<Box<dyn Future<Output = Result<()>> + Send>>` - Returns a future that resolves to
    ///   [`core::result::Result::Ok(())`] if the cleanup process is successful, otherwise returns
    ///   an error.
    #[instrument(skip(self), parent=&self.span)]
    fn cleanup(self) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
    where
        Self: 'static + Send + Sync + Sized,
    {
        Box::pin(async move {
            info!("Cleaning up task..");
            // If there is ongoing background work, abort it
            if let Some(handle) = self.maybe_bg_handle {
                debug!("Aborting background work..");
                handle.abort();
            }
            if let Some(handle) = self.maybe_fg_handle {
                debug!("Aborting spawned foreground work..");
                handle.abort();
            }
            // Wait until all references to the Arc have been dropped
            while Arc::strong_count(&self.inner) > 1 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            // Unwrap the inner activity of the task
            let mutex = Arc::try_unwrap(self.inner)
                .unwrap_or_else(|_| panic!("This is a race condition that should be fixed"));
            let inner = mutex.into_inner();
            // Clean up the inner activity
            debug!("Cleaning up Activity..");
            inner.cleanup().await?;
            // Log a message indicating that the cleanup process is successful
            info!("Task cleaned up");
            Ok(())
        })
    }

    /// This function is responsible for recovering a task after a recoverable error.
    /// It logs a message and returns the task.
    ///
    /// # Arguments
    ///
    /// * `self` - The instance of the task.
    ///
    /// # Returns
    ///
    /// * `Pin<Box<dyn Future<Output = Result<Self>> + Send>>` - Returns a future that resolves to
    ///   the task.
    fn recover(self) -> Pin<Box<dyn Future<Output = Result<Self>> + Send>>
    where
        Self: 'static + Send + Sync + Sized,
    {
        Box::pin(async move { Ok(self) })
    }
}

impl<T> Task<T>
where
    T: Activity,
{
    /// This function is responsible for getting the context of a task.
    /// It creates a new [`ActivityContext`] with the task's name, metrics, DNS, event bus, and a
    /// shutdown sender.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the instance of the task.
    /// * `tx` - An [mpsc::Sender] for the shutdoown signal of the task, so that the task
    /// can be shutdown programmatically from the activity, essentially shutting itself
    ///
    /// # Returns
    ///
    /// * `Arc<ActivityContext>` - The context of the task.
    fn get_context(&self) -> Arc<ActivityContext> {
        Arc::new(ActivityContext::new(
            self.name.clone(),
            self.metrics.clone(),
            self.dns.clone(),
            self.event_bus.get_copy(),
            self.internal_shutdown_tx.clone(),
        ))
    }

    /// This function is responsible for setting the foreground working state of a task.
    /// It updates the `fg_working` field of the task, sets the activity state in the metrics, and
    /// publishes an activity working event.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the instance of the task.
    /// * `working: bool` - The new foreground working state of the task.
    ///
    /// # Returns
    ///
    /// * `Result<(), TaskError>` - Returns [`core::result::Result::Ok(())`] if the operation is
    ///   successful, otherwise returns [`TaskError`].
    pub fn set_working_fg(&mut self, working: bool) -> Result<(), TaskError> {
        self.fg_working = working.into();
        self.metrics.set_activity_state_fg(&self.name, working);
        self.publish_activity_working(self.fg_working.load(Relaxed), self.bg_working.load(Relaxed))
    }

    /// This function is responsible for setting the background working state of a task.
    /// It updates the `bg_working` field of the task, sets the activity state in the metrics, and
    /// publishes an activity working event.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the instance of the task.
    /// * `working: bool` - The new background working state of the task.
    ///
    /// # Returns
    ///
    /// * `Result<(), TaskError>` - Returns [`core::result::Result::Ok(())`] if the operation is
    ///   successful, otherwise returns [`TaskError`].
    pub fn set_working_bg(&mut self, working: bool) -> Result<(), TaskError> {
        self.bg_working = working.into();
        self.metrics.set_activity_state_bg(&self.name, working);
        self.publish_activity_working(self.fg_working.load(Relaxed), self.bg_working.load(Relaxed))
    }

    /// This function is responsible for publishing an activity working event.
    /// It creates a new [`ActivityState`] with the foreground and background working states,
    /// serializes it into a JSON value, creates a new event with the JSON value as the body, and
    /// publishes the event to the event bus.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the instance of the task.
    /// * `fg_working: bool` - The foreground working state of the task.
    /// * `bg_working: bool` - The background working state of the task.
    ///
    /// # Returns
    ///
    /// * `Result<(), TaskError>` - Returns [`core::result::Result::Ok(())`] if the operation is
    ///   successful, otherwise returns [`TaskError`].
    fn publish_activity_working(
        &mut self,
        fg_working: bool,
        bg_working: bool,
    ) -> Result<(), TaskError> {
        let state =
            ActivityState { background_working: bg_working, foreground_working: fg_working };
        let body = serde_json::to_value(state).unwrap();
        let event = EventBuilder::new()
            .with_origin(self.id)
            .with_body(body)
            .with_type(EventType::ActivityStateChange)
            .build();
        self.event_bus.publish(event)?;
        Ok(())
    }

    /// This function is responsible for executing work commands on a task.
    /// It determines the type of work to be performed (non-blocking, blocking, or none) and
    /// executes the work accordingly. If the work type is non-blocking, it directly executes
    /// the work on the task. If the work type is blocking, it spawns a new blocking task to
    /// execute the work. If the work type is none, it panics as this is considered a bug.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the instance of the task.
    /// * `cmd: [`Vec<T::Command>`] - A vector of commands to be executed on the task.
    ///
    /// # Returns
    ///
    /// * `Result<Option<JoinHandle<eyre::Result<()>>>, TaskError>` - Returns a future representing
    ///   the ongoing work if the work type is blocking, otherwise returns None. If an error occurs
    ///   while executing the work, it returns a [`TaskError`].
    #[instrument(name = "do_work", skip(self, cmd, context), level="info", parent= &self.span)]
    async fn do_work(
        &mut self,
        cmd: Vec<T::Command>,
        context: WorkContext,
    ) -> Result<Option<JoinHandle<eyre::Result<()>>>, TaskError> {
        // Get a lock on the activity
        let mut activity = self.inner.clone().lock_owned().await;
        let res = match activity.work_type() {
            // If the work type is non-blocking, directly execute the work on the task.
            FgWorkType::NonBlocking => {
                debug!("[Foreground Work] Start non-blocking work");
                let span = info_span!("do_work");
                activity
                    .do_work(cmd, context)
                    .instrument(span)
                    .await
                    .map_err(|e| TaskError::FgWork { source: e })?;
                debug!("[Foreground Work] Finish non-block work");
                None
            }
            // If the work type is blocking, spawn a new blocking task to execute the work.
            FgWorkType::SpawnBlocking => {
                debug!("[Foreground Work] Spawn blocking work");
                let span = info_span!("do_work_blocking");
                // Set task status to 'foreground working'
                self.set_working_fg(true)?;
                let handle = tokio::task::spawn_blocking(move || {
                    let _enter = span.enter();

                    activity.do_work_blocking(cmd, context)
                });
                Some(handle)
            }
            // If the work type is non-blocking, spawn a new non-blocking task to execute the work.
            FgWorkType::SpawnNonBlocking => {
                debug!("[Foreground Work] Spawn non-blocking work");
                let span = info_span!("do_work");
                let handle = tokio::task::spawn(
                    async move { activity.do_work(cmd, context).await }.instrument(span),
                );
                Some(handle)
            }
            // If the work type is none, panic as this is considered a bug.
            FgWorkType::None => {
                debug!("[Foreground Work] The Task doesn't do any work, but instead idles");
                None
            }
        };
        Ok(res)
    }
}

///
impl<T> Display for Task<T>
where
    T: Activity,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.id)
    }
}

impl<T> Debug for Task<T>
where
    T: Send + Sync + Display + Debug + Activity,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({}", self.name, self.id)
    }
}
/// The state of the activity. Whether it's working on the `foreground` or the `background`.
#[derive(Deserialize, Serialize)]
pub struct ActivityState {
    /// The activity is working on the background. Background work is usually work that is not the
    /// result of some event from the [`EventBus`]
    pub background_working: bool,
    /// The activity is working on the foreground. Foreground work is the direct result of the
    /// [`Task`] reading an event from the [`EventBus`]
    pub foreground_working: bool,
}

#[derive(Clone)]
pub struct WorkContext {
    pub activity: Arc<ActivityContext>,
    pub event: EventContext,
}

impl WorkContext {
    pub fn new(activity: Arc<ActivityContext>, event: impl Into<EventContext>) -> Self {
        Self { activity, event: event.into() }
    }

    pub fn into_erased_map(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.extend(self.event.into_erased_map());
        map.extend(self.activity.get_erased_map());
        map
    }

    pub fn into_map(self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.extend(self.event.into_map());
        map.extend(self.activity.get_map());
        map
    }
}
impl Debug for WorkContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkContext")
            .field("activity", &self.activity)
            .field("event", &self.event)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct EventContext {
    pub(crate) event: Event,
}

impl EventContext {
    pub fn new(event: Event) -> Self {
        Self { event }
    }
    pub fn into_erased_map(self) -> HashMap<String, String> {
        self.event.into_jq_hashmap_erased()
    }

    pub fn into_map(self) -> HashMap<String, Value> {
        self.event.into_jq_hashmap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        activities::Activity,
        mocks::mock_activity::{NonBlockingMockActivity, SpawnNonBlockingMockActivity},
        tasks::subscriptions::Subscription,
    };
    use serde_json::json;
    use std::{
        str::FromStr,
        sync::{atomic::Ordering, Arc},
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::sync::Mutex;
    use tracing_test::traced_test;

    fn setup_task<T: Activity>(inner: T) -> Task<T> {
        Task::<T> {
            name: "TestTask".to_string(),
            id: 1,
            span: Span::current(),
            metrics: MetricsRegistry::new(),
            category: TaskCategory::UserDefined,
            event_bus: EventBus::new(),
            dns: Arc::new(TaskDns::new()),
            subscriptions: Subscriptions::new(),
            buffer: EventBuffer::new(),
            inner: Arc::new(Mutex::new(inner)),
            fg_working: AtomicBool::new(false),
            bg_working: AtomicBool::new(false),
            maybe_bg_handle: None,
            maybe_fg_handle: None,
            internal_shutdown_tx: broadcast::channel(1).0,
        }
    }

    #[test]
    fn test_task_category_from_str() {
        let category = TaskCategory::from_str("userdefined").unwrap();
        assert_eq!(category, TaskCategory::UserDefined);

        let category = TaskCategory::from_str("system").unwrap();
        assert_eq!(category, TaskCategory::System);

        let category = TaskCategory::from_str("unknown");
        assert!(category.is_err());
    }

    #[tokio::test]
    async fn test_set_working_fg() {
        let (inner, _, _) = SpawnNonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        task.set_working_fg(true).unwrap();
        assert!(task.fg_working.load(Ordering::Relaxed));
    }

    #[test]
    fn test_set_working_bg() {
        let (inner, _, _) = SpawnNonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        task.set_working_bg(true).unwrap();
        assert!(task.bg_working.load(Ordering::Relaxed));
    }

    // The `new` function initializes a new `EventBus` with a capacity of 100.
    #[test]
    fn test_new_function_initializes_event_bus_with_capacity_100() {
        let event_bus = EventBus::new();
        assert_eq!(event_bus.free_percentage(), 100);
    }

    #[tokio::test]
    async fn test_publish_function_publishes_event_with_timestamp_and_id() {
        let mut event_bus = EventBus::new();
        let event = EventBuilder::new().with_origin(13242).build();
        event_bus.publish(event.clone()).unwrap();
        let received_event = event_bus.read().await.unwrap();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect(
            "Time went
backwards",
        );
        assert_eq!(timestamp.as_secs(), received_event.get_timestamp());
        assert_eq!(event.get_timestamp(), 0);
        assert_eq!(received_event.id, 1);
    }

    #[tokio::test]
    async fn test_publish_function_not_returns_error_when_event_bus_is_full() {
        let mut event_bus = EventBus::new();
        for _ in 0..100 {
            let event = EventBuilder::new().build();
            event_bus.publish(event).unwrap();
        }
        let event = EventBuilder::new().build();
        let result = event_bus.publish(event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_id_function_increments_counter_and_returns_new_id() {
        let mut event_bus = EventBus::new();
        let initial_counter = event_bus.counter;
        let event = EventBuilder::new().build();
        let id = event_bus.get_id(&event);
        assert_eq!(id, initial_counter + 1);
    }

    #[tokio::test]
    async fn test_bootstrap() {
        let (inner, status_rx, _internal_tx) = SpawnNonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let context = task.get_context();

        let status_before = status_rx.borrow().to_owned();
        assert!(!status_before.bootstraped);
        assert!(context.dns.find_id_from_name("TestTask").is_none());

        assert!(task.bootstrap(context.clone()).await.is_ok());
        let status_after = status_rx.borrow().to_owned();
        assert!(context.dns.find_id_from_name("TestTask").is_some());
        assert!(status_after.bootstraped);
    }

    #[tokio::test]
    async fn test_read_event_bus() {
        let (inner, _status_rx, _internal_tx) = NonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let mut event_bus = task.event_bus.get_copy();
        let mut event = EventBuilder::new().with_body(json!("hey o")).build();
        event_bus.publish(event.clone()).unwrap();
        event.id = 1;
        event.timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let result = task.read_event_bus().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), event);
    }

    #[tokio::test]
    async fn test_send_event() {
        let (inner, _status_rx, _internal_tx) = NonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let mut event_bus = task.event_bus.get_copy();
        let mut event = EventBuilder::new().with_body(json!("hey o")).build();
        task.send_event(event.clone()).unwrap();
        event.id = 1;
        event.origin = task.id;
        event.timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let result = event_bus.read().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), event);
    }

    #[tokio::test]
    async fn test_handle_event_and_subscribed_and_working() {
        let (inner, _status_rx, _internal_tx) = NonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let rule = "origin == 13";
        let sub = Subscription::new(rule, Default::default());
        task.subscriptions.add(sub);
        let activity_context = task.get_context();
        let context = WorkContext::new(activity_context, EventBuilder::new().build());
        task.set_working_fg(true).unwrap();
        let event = EventBuilder::new().with_body(json!("hey o")).with_origin(13).build();
        task.send_event(event.clone()).unwrap();
        let result = task.handle_event(event.clone(), context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert_eq!(task.buffer.pop().unwrap(), event);
    }

    #[tokio::test]
    async fn test_handle_event_and_not_subscribed() {
        let (inner, _status_rx, _internal_tx) = NonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let rule = "origin == 13";
        let sub = Subscription::new(rule, Default::default());
        task.subscriptions.add(sub);
        let activity_context = task.get_context();
        let context = WorkContext::new(activity_context, EventBuilder::new().build());
        let event = EventBuilder::new().with_body(json!("hey o")).with_origin(13).build();
        task.send_event(event.clone()).unwrap();
        let result = task.handle_event(event.clone(), context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert_eq!(task.buffer.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[traced_test]
    async fn test_handle_event_and_subscribed_and_not_working() {
        let (inner, status_rx, _internal_tx) = SpawnNonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let rule = "origin == 13";
        let sub = Subscription::new(rule, Default::default());
        task.subscriptions.add(sub);
        let activity_context = task.get_context();
        let context = WorkContext::new(activity_context, EventBuilder::new().build());
        let event = EventBuilder::new().with_body(json!("hey o")).with_origin(13).build();
        // Mock Acitivty decides a command if buffer.len() % 2 == 0
        task.buffer.push(event.clone()).unwrap();
        assert_eq!(task.buffer.len(), 1);
        let result = task.handle_event(event.clone(), context).await;
        // we await the spawned task so that it has time to update the invocations watch
        assert!(result.unwrap().unwrap().await.is_ok());
        let status = status_rx.borrow().to_owned();
        assert_eq!(task.buffer.len(), 0);
        assert_eq!(status.decide_invocations, 1);
        assert_eq!(status.work_invocations, 1);
    }

    #[tokio::test]
    async fn test_is_subscribed_and_ready() {
        let (inner, _status_rx, _internal_tx) = SpawnNonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let rule = "origin == 13";
        let sub = Subscription::new(rule, Default::default());
        task.subscriptions.add(sub);

        let event = EventBuilder::new().with_body(json!("hey o")).with_origin(13).build();
        let result = task.is_subscribed_and_ready(&event);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let event = EventBuilder::new().with_body(json!("hey o")).with_origin(1123).build();
        let result = task.is_subscribed_and_ready(&event);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_store_event_to_buffer() {
        let (inner, _status_rx, _internal_tx) = SpawnNonBlockingMockActivity::new();
        let mut task = setup_task(inner);
        let event = EventBuilder::new().build();
        let result = task.store_event_to_buffer(event.clone());
        assert!(result.is_none());
        assert_eq!(task.buffer.len(), 1);
        assert_eq!(task.buffer.pop().unwrap(), event);
    }
}
