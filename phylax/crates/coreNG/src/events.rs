use eyre::{Report, Result};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fmt,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    str::FromStr,
};
use tokio::sync::*;

use crate::{error::EventBusError, tasks::task::EventContext};

/// The Event is a core part of Phylax. It's emitted by events in the [`EventBus`] and
/// [`super::tasks::task::Task`] can subscribe to it by defining a
/// [`super::tasks::subscriptions::Subscription`].
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// The id of the task that emitted the event
    pub origin: u32,
    /// The type of the event
    pub event_type: EventType,
    /// The body of the event
    pub body: EventBody,
    /// The id of the event
    pub id: u64,
    /// The UNIX timestamp at which the event was emitted
    pub timestamp: u64,
}

/// It builds an event
pub struct EventBuilder {
    event: Event,
}

/// Implementation of the EventBuilder struct
impl EventBuilder {
    /// Creates a new EventBuilder with a default Event
    pub fn new() -> Self {
        EventBuilder { event: Event::default() }
    }

    /// Sets the origin of the event
    ///
    /// # Arguments
    ///
    /// * `origin` - A u32 that holds the origin of the event
    pub fn with_origin(mut self, origin: u32) -> Self {
        self.event.origin = origin;
        self
    }

    /// Sets the type of the event
    ///
    /// # Arguments
    ///
    /// * `event_type` - An EventType that holds the type of the event
    pub fn with_type(mut self, event_type: impl Into<EventType>) -> Self {
        self.event.event_type = event_type.into();
        self
    }

    /// Sets the body of the event
    ///
    /// # Arguments
    ///
    /// * `body` - A Value that holds the body of the event
    pub fn with_body(mut self, body: impl Into<Value>) -> Self {
        self.event.body = EventBody::new(body);
        self
    }

    /// Builds the event and returns it
    pub fn build(self) -> Event {
        self.event
    }
}

impl Default for EventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EventBody(pub Value);

// PartialEq is implemented manually because the default implementation
// of PartialEq for dyn Debug is not implemented
// This is not performant, but it is only used for testing
impl PartialEq for EventBody {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

impl EventBody {
    pub fn empty() -> Self {
        EventBody(Value::Null)
    }

    pub fn new(value: impl Into<Value>) -> Self {
        EventBody(value.into())
    }
}

impl Default for EventBody {
    fn default() -> Self {
        EventBody::empty()
    }
}

impl Debug for EventBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Implementation of Event struct
impl Event {
    /// Returns the event type
    pub fn get_type(&self) -> &EventType {
        &self.event_type
    }

    /// Returns the origin of the event
    pub fn get_origin(&self) -> u32 {
        self.origin
    }

    /// Sets the origin of the event and returns the event
    pub fn set_origin(&mut self, id: u32) {
        self.origin = id;
    }

    /// Consumes the event and returns the body
    pub fn get_body(self) -> Value {
        self.body.0
    }

    /// Returns a reference to the body of the event
    pub fn get_body_ref(&self) -> &Value {
        &self.body.0
    }

    /// Returns the timestamp of the event
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Sets the timestamp of the event
    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    /// Sets the id of the event
    pub fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    /// Converts the event into a hashmap
    pub fn into_jq_hashmap(self) -> HashMap<String, serde_json::Value> {
        let mut context = HashMap::new();
        let event = serde_json::to_value(self).expect("event is not serializable. This is a bug");
        event_to_jq_hashmap("event", &event, &mut context);
        context
    }

    pub fn into_jq_hashmap_erased(self) -> HashMap<String, String> {
        let mut context = HashMap::new();
        let event = serde_json::to_value(self).expect("event is not serializable. This is abug");
        event_to_jq_hashmap_erased("event", &event, &mut context);
        context
    }
}

/// The id of a group of events, which all are of the same type and are emitted from the same event.
/// With the addition of dynamic and flexible subscriptions, this should be deprecated soon.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct EventId {
    /// The ID of the task that emits the event
    origin: u32,
    /// The type of the event
    event_type: EventType,
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// A new block was emitted. The body of the task has more information about the block
    NewBlock,
    /// A task should be activated. The body of the task defines which tasks should be activated
    ActivateTask,
    /// An alert is firing
    AlertFiring,
    /// A task changed state. This is emitted by the
    /// [`super::activities::system::orchestrator::Orchestrator`] when the status
    /// ([`sisyphus_tasks::sisyphus::TaskStatus`]) of a task changes
    TaskStatusChange,
    /// The state of a task's activity changed. This is emitted when the activity changes
    /// [`super::tasks::task::ActivityState`], either for a foreground or a background work
    ActivityStateChange,
    /// An alert was just executed succesfully. The body of the event has details about the
    /// execution.
    AlertExecution,
    /// An action was just executed succesfully. The body of the event has details about the
    /// execution.
    ActionExecution,
    /// The config file or repository was changed.
    ConfigChanged,
    /// The API received some command, usually to restart Phylax with or without a new config.
    ApiCommand,
    #[default]
    /// Generic event. Placeholder.
    Info,
}

impl Display for EventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EventType::NewBlock => write!(f, "new_block"),
            EventType::AlertFiring => write!(f, "alert_firing"),
            EventType::ActivateTask => write!(f, "activate_task"),
            EventType::TaskStatusChange => write!(f, "task_state_change"),
            EventType::ActivityStateChange => write!(f, "activity_state_change"),
            EventType::AlertExecution => write!(f, "alert_execution"),
            EventType::ActionExecution => write!(f, "action_execution"),
            EventType::ApiCommand => write!(f, "api_command"),
            EventType::Info => write!(f, "info"),
            EventType::ConfigChanged => write!(f, "config_changed"),
        }
    }
}

impl FromStr for EventType {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "new_block" => Ok(EventType::NewBlock),
            "alert_firing" => Ok(EventType::AlertFiring),
            "activate_task" => Ok(EventType::ActivateTask),
            "task_state_change" => Ok(EventType::TaskStatusChange),
            "activity_state_change" => Ok(EventType::ActivityStateChange),
            "alert_execution" => Ok(EventType::AlertExecution),
            "action_execution" => Ok(EventType::ActionExecution),
            "api_command" => Ok(EventType::ApiCommand),
            "info" => Ok(EventType::Info),
            "config_changed" => Ok(EventType::ConfigChanged),
            _ => Err(Report::msg(format!("'{}' is not a valid event type", s))),
        }
    }
}

#[derive(Debug)]
pub struct EventBus {
    pub bus_tx: broadcast::Sender<Event>,
    pub bus_rx: broadcast::Receiver<Event>,
    pub counter: u64,
}

impl AsRef<EventBus> for EventBus {
    fn as_ref(&self) -> &EventBus {
        self
    }
}

impl EventBus {
    const CAPACITY: usize = 100;
    pub fn new() -> Self {
        let (bus_tx, bus_rx) = broadcast::channel(Self::CAPACITY);
        EventBus { bus_tx, bus_rx, counter: 0 }
    }

    pub fn publish<T: Into<Event>>(&mut self, event: T) -> Result<(), EventBusError> {
        let mut event = event.into();
        event.set_timestamp(chrono::prelude::Utc::now().timestamp() as u64);
        event.set_id(self.get_id(&event));
        self.bus_tx.send(event)?;
        Ok(())
    }

    pub async fn read(&mut self) -> Result<Event, EventBusError> {
        let event = self.bus_rx.recv().await?;
        Ok(event)
    }

    pub fn get_copy(&self) -> Self {
        EventBus {
            bus_tx: self.bus_tx.clone(),
            bus_rx: self.bus_tx.subscribe(),
            counter: self.counter,
        }
    }

    pub fn get_id(&mut self, _event: &Event) -> u64 {
        self.counter += 1;
        self.counter
    }

    /// Calculate the free percentage of the event bus.
    pub fn free_percentage(&self) -> usize {
        (Self::CAPACITY - self.bus_rx.len()) * (100 / Self::CAPACITY)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EVENT-[ Origin:\"{}\" | Type:\"{}\" | B:<..> ]",
            self.get_origin(),
            self.get_type(),
        )
    }
}

impl Debug for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EVENT-[ ID: \"{}\" | Origin:\"{}\" | Type:\"{}\" | Time {} | Body:\"{:?}\" ]",
            self.id,
            self.get_origin(),
            self.get_type(),
            self.timestamp,
            self.get_body_ref()
        )
    }
}

/// Hash a string into a number. It uses the [`DefaultHasher`].
pub fn hash_string(s: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() as u32
}

/// Helper function to add a value to the context
///
/// # Arguments
///
/// * `prefix` - A string slice that holds the prefix
/// * `value` - A reference to a serde_json::Value that holds the value
/// * `context` - A mutable reference to a HashMap that holds the context
fn event_to_jq_hashmap(
    prefix: &str,
    value: &serde_json::Value,
    context: &mut HashMap<String, serde_json::Value>,
) {
    context.insert(prefix.to_owned(), value.clone());
    match value {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                let new_key = if prefix.is_empty() && !key.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                };
                event_to_jq_hashmap(&new_key, value, context);
            }
        }
        serde_json::Value::Array(arr) => {
            for (index, value) in arr.iter().enumerate() {
                let new_key = format!("{}[{}]", prefix, index);
                event_to_jq_hashmap(&new_key, value, context);
            }
        }
        _ => {}
    };
}

fn event_to_jq_hashmap_erased(
    prefix: &str,
    value: &serde_json::Value,
    context: &mut HashMap<String, String>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                let new_key = if prefix.is_empty() && !key.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                };
                event_to_jq_hashmap_erased(&new_key, value, context);
            }
        }
        serde_json::Value::Array(arr) => {
            for (index, value) in arr.iter().enumerate() {
                let new_key = format!("{}[{}]", prefix, index);
                event_to_jq_hashmap_erased(&new_key, value, context);
            }
        }
        serde_json::Value::Number(n) => {
            let value = n
                .as_i64()
                .map(|i| i.to_string())
                .or_else(|| n.as_f64().map(|f| f.to_string()))
                .unwrap();
            context.insert(prefix.to_owned(), value);
        }
        serde_json::Value::String(s) => {
            context.insert(prefix.to_owned(), s.clone());
        }
        serde_json::Value::Bool(b) => {
            context.insert(prefix.to_owned(), b.to_string());
        }
        _ => {}
    };
}

impl From<Event> for EventContext {
    fn from(event: Event) -> Self {
        Self { event }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // The `publish` function publishes an event to the event bus, setting the timestamp and ID of
    // the event.
    #[test]
    fn test_publish_event() {
        let mut event_bus = EventBus::new();
        let event = Event::default();
        event_bus.publish(event).unwrap();
        assert_eq!(event_bus.bus_rx.len(), 1);
        assert_eq!(event_bus.counter, 1);
    }

    // The `read` function asynchronously reads an event from the event bus.
    #[tokio::test]
    async fn test_read_event() {
        let mut event_bus = EventBus::default();
        let mut event = Event::default();
        event_bus.publish(event.clone()).unwrap();
        event.set_timestamp(chrono::prelude::Utc::now().timestamp() as u64);
        event.set_id(1);
        let result = event_bus.read().await.unwrap();
        assert_eq!(result, event);
    }

    // Publishing an event when the event bus is full should return an error.
    #[tokio::test]
    async fn test_publish_event_when_full() {
        let mut event_bus = EventBus::new();
        for _ in 0..EventBus::CAPACITY + 100 {
            let event = Event::default();
            event_bus.publish(event).unwrap();
        }
        let event = Event::default();
        let _ = event_bus.publish(event);
        match event_bus.read().await {
            Err(EventBusError::RecvError(internal)) => {
                let tokio_error = tokio::sync::broadcast::error::RecvError::Lagged(73);
                assert_eq!(internal, tokio_error);
            }
            _other => {
                panic!("she ")
            }
        }
    }

    #[tokio::test]
    async fn test_get_copy() {
        let mut event_bus = EventBus::new();
        let mut copy = event_bus.get_copy();
        let event = Event::default();
        match event_bus.publish(event) {
            Ok(_) => (),
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
        // Since `tokio::sync::broadcast::Sender` does not implement `PartialEq`, we cannot directly
        // compare `bus_tx`. Instead, we can verify that both `bus_tx` are subscribed to the
        // same channel by sending a new event and checking if both `bus_rx` receive it.
        let mut new_event = Event::default();
        new_event.set_timestamp(chrono::prelude::Utc::now().timestamp() as u64);
        match copy.publish(new_event.clone()) {
            Ok(_) => (),
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
        let event_bus_event = match event_bus.read().await {
            Ok(event) => event,
            Err(e) => panic!("Unexpected error: {:?}", e),
        };
        let copy_event = match copy.read().await {
            Ok(event) => event,
            Err(e) => panic!("Unexpected error: {:?}", e),
        };
        // First event has id of 1
        new_event.set_id(1);
        assert_eq!(event_bus_event, new_event);
        assert_eq!(copy_event, new_event);
        assert_eq!(copy.bus_rx.len(), event_bus.bus_rx.len());
        assert_eq!(copy.counter, event_bus.counter);
    }
    #[test]
    fn test_event_builder() {
        let origin = 1;
        let event_type = EventType::NewBlock;
        let body = Value::String("test".to_string());

        let event = EventBuilder::new()
            .with_origin(origin)
            .with_type(event_type)
            .with_body(body.clone())
            .build();

        assert_eq!(event.origin, origin);
        assert_eq!(event.event_type, event_type);
        assert_eq!(event.body.0, body);
    }

    #[test]
    fn test_event_methods() {
        let mut event = Event::default();
        let origin = 1;
        let timestamp = 1234567890;
        let id = 2;

        event.set_origin(origin);
        event.set_timestamp(timestamp);
        event.set_id(id);

        assert_eq!(event.id, id);
        assert_eq!(event.get_origin(), origin);
        assert_eq!(event.get_timestamp(), timestamp);
    }

    #[test]
    fn test_event_bus_methods() {
        let mut event_bus = EventBus::new();
        let event = Event::default();
        // increment 1
        let id = event_bus.get_id(&event);

        assert_eq!(id, 1);
        assert_eq!(event_bus.free_percentage(), 100);

        // id is incremented again
        event_bus.publish(event.clone()).unwrap();
        assert_eq!(event_bus.free_percentage(), 99);
        // id is incremented again
        assert_eq!(event_bus.get_id(&event), 3);
    }

    #[test]
    fn test_event_type_conversion() {
        let event_type_str = "new_block";
        let event_type = EventType::from_str(event_type_str).unwrap();

        assert_eq!(event_type, EventType::NewBlock);
        assert_eq!(event_type.to_string(), event_type_str);
    }

    #[test]
    fn test_event_body_methods() {
        let body = EventBody::new(Value::String("test".to_string()));
        let empty_body = EventBody::empty();

        assert_eq!(body.0, Value::String("test".to_string()));
        assert_eq!(empty_body.0, Value::Null);
        assert_eq!(EventBody::default().0, Value::Null);
        assert_ne!(body, empty_body);
    }

    #[test]
    fn test_hash_string() {
        let s = "test";
        let hash = hash_string(s);

        assert_eq!(hash, 854508108); // This is the expected hash for "test"
    }

    #[test]
    fn test_event_id() {
        let event_id = EventId { origin: 1, event_type: EventType::NewBlock };

        assert_eq!(event_id.origin, 1);
        assert_eq!(event_id.event_type, EventType::NewBlock);
    }

    #[test]
    fn test_display_and_debug_implementations() {
        let event = Event::default();
        assert_eq!(format!("{}", event), "EVENT-[ Origin:\"0\" | Type:\"info\" | B:<..> ]");
        assert_eq!(
            format!("{:?}", event),
            "EVENT-[ ID: \"0\" | Origin:\"0\" | Type:\"info\" | Time 0 | Body:\"Null\" ]"
        );

        let event_type = EventType::NewBlock;
        assert_eq!(format!("{}", event_type), "new_block");
        assert_eq!(format!("{:?}", event_type), "NewBlock");

        let event_id = EventId { origin: 1, event_type: EventType::NewBlock };
        assert_eq!(format!("{:?}", event_id), "EventId { origin: 1, event_type: NewBlock }");
    }

    #[test]
    fn test_get_copy_function_not_preserve_counter_value() {
        let mut event_bus = EventBus::new();
        let event = EventBuilder::new().build();
        event_bus.publish(event).unwrap();
        assert_eq!(event_bus.bus_rx.len(), 1);
        assert_eq!(event_bus.free_percentage(), 99);
        let copy_event_bus = event_bus.get_copy();
        assert_eq!(copy_event_bus.free_percentage(), 100);
    }

    #[test]
    fn test_get_id_function_increments_counter_and_returns_new_id() {
        let mut event_bus = EventBus::new();
        let initial_counter = event_bus.counter;
        let event = EventBuilder::new().build();
        let id = event_bus.get_id(&event);
        assert_eq!(id, initial_counter + 1);
    }

    #[test]
    fn test_event_into_jq_hashmap() {
        let value = json!({
            "key1": "value1",
            "key2": {
                "subkey1": "subvalue1",
                "subkey2": [1, 2, 3]
            },
            "key3": true
        });
        let event = EventBuilder::new().with_body(value).build();
        let map = event.into_jq_hashmap();
        assert_eq!(map.get("event.body.key1").unwrap(), &json!("value1"));
        assert_eq!(map.get("event.body.key2.subkey1").unwrap(), &json!("subvalue1"));
        assert_eq!(map.get("event.body.key2.subkey2[0]").unwrap(), &json!(1));
        assert_eq!(map.get("event.body.key2.subkey2").unwrap(), &json!([1, 2, 3]));
        assert_eq!(map.get("event.body.key3").unwrap(), &json!(true));
    }
}
