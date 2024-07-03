use std::{
    fmt::Debug,
    sync::{atomic::AtomicU64, Arc},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{
    self,
    error::{RecvError, SendError},
};

use crate::error::EventBusError;

/// Represents an event bus used for sending and receiving events.
///
/// The `EventBus` struct fields are private, and it exposes them through helper methods that
/// automatically decorate its outputs. Internally, it contains a sender and receiver for
/// broadcasting events, along with a counter to track event IDs.
///
/// # Fields
///
/// * `bus_sender` - The sender channel for broadcasting events.
/// * `bus_receiver` - The receiver channel for receiving events.
/// * `counter` - An atomic counter to generate unique event IDs.
///
/// # Examples
///
/// ```
/// # tokio_test::block_on(async {
/// use phylax_interfaces::event::{EventBus, EventType};
///
/// let mut event_bus = EventBus::new();
///
/// // Send an event without checking whether all receivers have been dropped
/// event_bus.send_unchecked(EventType::Info { info_msg: "Hello, world!".to_string() });
///
/// // Receive an event asynchronously
/// let received_event = event_bus.receive().await;
/// let info_event_body = received_event.unwrap().body;
/// assert_eq!(info_event_body, EventType::Info { info_msg: "Hello, world!".to_string() });
/// # })
/// ```
#[derive(Debug)]
pub struct EventBus {
    bus_sender: broadcast::Sender<Event>,
    bus_receiver: broadcast::Receiver<Event>,
    counter: Arc<AtomicU64>,
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            bus_sender: self.bus_sender.clone(),
            bus_receiver: self.bus_receiver.resubscribe(),
            counter: self.counter.clone(),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    const DEFAULT_CAPACITY: usize = 100;

    /// Create a new EventBus with the event id counter set to 0.
    pub fn new() -> Self {
        let (bus_tx, bus_rx) = broadcast::channel(Self::DEFAULT_CAPACITY);
        EventBus { bus_sender: bus_tx, bus_receiver: bus_rx, counter: Arc::new(AtomicU64::new(0)) }
    }

    /// Asynchronously receives an event from the event bus.
    ///
    /// # Errors
    ///
    /// Returns an error in the following cases:
    /// - `EventBusError::CriticalError`: If all senders have been dropped, indicating a critical
    ///   error.
    /// - `EventBusError::SkippedMessageError`: If the receiver is skipping messages due to lag.
    pub async fn receive(&mut self) -> Result<Event, EventBusError> {
        match self.bus_receiver.recv().await {
            // Everything worked out ok
            Ok(event) => Ok(event),
            // All senders have been dropped
            Err(RecvError::Closed) => {
                Err(EventBusError::CriticalError(Box::new(RecvError::Closed)))
            }
            // Receiver is skipping messages
            Err(RecvError::Lagged(count)) => {
                Err(EventBusError::SkippedMessageError(RecvError::Lagged(count)))
            }
        }
    }

    /// Sends an event to the event bus without checking for errors.
    ///
    /// This method sends an event of the specified type to the event bus without performing any
    /// error handling, which arise when all receivers have been dropped. It is recommended to
    /// use `send` method for error-checked event sending.
    pub fn send_unchecked(&self, event_type: EventType) {
        let _ = self.send(event_type);
    }

    /// Sends an event to the event bus.
    ///
    /// This method sends an event of the specified type to the event bus and returns a result
    /// indicating whether the event was successfully sent or if an error occurred.
    pub fn send(&self, event_type: EventType) -> Result<(), EventBusError> {
        // Increment the counter with relaxed ordering because we only expect commutative addition
        let id = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let event = Event::new(event_type, id);
        match self.bus_sender.send(event) {
            // Everything worked out ok
            Ok(_) => Ok(()),
            // All receivers have been dropped
            Err(SendError(e)) => Err(EventBusError::CriticalError(Box::new(SendError(e)))),
        }
    }

    pub fn free_percentage(&self) -> usize {
        (Self::DEFAULT_CAPACITY - self.bus_receiver.len()) * (100 / Self::DEFAULT_CAPACITY)
    }
}

/// Represents an event in the event bus.
///
/// This struct contains information about the event type, unique id, and the timestamp at which the
/// event was emitted.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    // The event type and its associated data
    #[serde(flatten)]
    pub body: EventType,
    // The unique id assigned to the event from the event bus
    id: u64,
    /// The UNIX timestamp at which the event was emitted
    timestamp: u128,
}

impl Event {
    pub(crate) fn new(body: EventType, id: u64) -> Self {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            // System time errors are consumed and events time defaults to 0
            .unwrap_or_default()
            .as_micros();
        Self { body, id, timestamp: epoch }
    }
}

/// Represents the type of event that can be sent through the event bus.
///
/// This enum defines various types of events that can occur within the system, such as NodeStartup,
/// NodeShutdown, TaskReceivedMessage, etc. as well as a custom event message type called `Info`.
/// Each variant contains specific data related to the event type, like task names, error messages,
/// and more.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
// When (de)serialized, creates an event_type field with the snake case variant name as the value
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum EventType {
    NodeStartup { task_names: Vec<String> },
    NodeShutdown(NodeShutdownCause),
    TaskReceivedMessage(TaskEventDetails),
    TaskSentMessage(TaskEventDetails),
    TaskSentNoMessage(TaskEventDetails),
    TaskRecoverableUserError(TaskEventDetails),
    TaskUnrecoverableUserTaskError(TaskEventDetails),
    TaskSystemError(TaskEventDetails),
    // TODO: Add when task lineage is improved
    //SubtaskCreated,
    //SubtaskCompleted,
    // TODO: Add when API is fleshed out
    //ApiTriggered(RouteAndEventDetails),
    Info { info_msg: String },
}

/// Represents the possible causes for a node shutdown event.
///
/// This enum defines the broad reasons that can lead to a node shutting down, such as user-code
/// triggered unrecoverable errors or shutdown signals, system-level errors and panics, or due to
/// the external signal returned by the node launcher being triggered.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeShutdownCause {
    /// Caused by signals or errors returned by a user-defined activity, like through
    /// UnrecoverableError.
    ActivitySignal,
    /// Caused by errors that originate from the system runtime itself.
    SystemError { error_msg: String },
    /// Shutdown sent by the external shutdown signal returned by the node.
    ExternalSignal,
}

/// Represents minimal data about a task for a specific task event, including the task name, task
/// type, and an optional error message.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEventDetails {
    /// The unique task name.
    pub task_name: String,
    /// The type of activity that ran the event (i.e. the const Activity name).
    pub task_type: String,
    /// The error message associated with the event.
    pub error_msg: Option<String>,
}

impl TaskEventDetails {
    /// Creates a new TaskEventDetails instance with the provided error message.
    pub fn with_error_msg(self, error_msg: String) -> Self {
        Self { task_name: self.task_name, task_type: self.task_type, error_msg: Some(error_msg) }
    }
}
