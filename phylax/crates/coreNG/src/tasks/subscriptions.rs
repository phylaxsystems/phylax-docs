use crate::events::{hash_string, Event};
use evalexpr::{
    Context, ContextWithMutableFunctions, ContextWithMutableVariables, EvalexprError, Function,
    HashMapContext,
};
use phylax_config::Trigger;
use phylax_tracing::tracing::{debug, error, trace};

/// Represents a collection of subscriptions.
#[derive(Debug)]
pub struct Subscriptions(Vec<Subscription>);

/// Represents a subscription.
/// A subscription describes when a task should activate to an event, according to an arbtrary
/// assertion or 'rule'.
#[derive(Debug, Clone)]
pub struct Subscription {
    /// The rule to which every event is evaluated against
    rule: String,
    /// A counter of the times the `rule` has evaluated to `True`. This enables the user to
    /// activate tasks every N events.
    counter: SubscriptionCounter,
}
impl Subscription {
    /// Creates a new Subscription.
    ///
    /// # Arguments
    ///
    /// * `rule` - A string that represents the rule to which every event is evaluated against.
    /// * `counter` - A SubscriptionCounter that counts the times the `rule` has evaluated to
    ///   `True`.
    ///
    /// # Returns
    ///
    /// * `Subscription` - The newly created Subscription.
    pub fn new(rule: impl ToString, counter: SubscriptionCounter) -> Self {
        Subscription { rule: rule.to_string(), counter }
    }
    /// Checks if an event is subscribed.
    ///
    /// It evaluates the event using the library [`evalexpr`] and sets the context of the evaluation
    /// [`evaluate_with_context`]. It adds as context all the attributes of an event, such as
    /// `origin` or `event_type`.
    ///
    /// # Arguments
    ///
    /// * `context` - A context that implements the `Context` trait.
    ///
    ///
    /// # Returns
    ///
    /// * `bool` - Returns `true` if the event is subscribed, `false` otherwise.
    pub fn is_subscribed(&mut self, context: &impl Context) -> Result<bool, SubscriptionsError> {
        match evalexpr::eval_with_context(&self.rule, context) {
            Ok(result) => {
                let eval = result.as_boolean().unwrap_or(false);
                debug!(rule = self.rule, ?result, "eval succeed");
                Ok(eval)
            }
            Err(err) => {
                // This error means that a variable in the rule was not found.
                // This is most likely because the rule expects another event (e.g only newBlock
                // events have the block number) Thus,we know for sure that the
                // subscription if false for the particular event. How can it be true
                // if the event doesn't even have a field that the rule expects.
                // There is a small edge case of the user mistake (e.g misspell), but there isn't a
                // way around this.
                if let EvalexprError::VariableIdentifierNotFound(_variable) = err {
                    Ok(false)
                } else {
                    Err(SubscriptionsError::Eval { source: err, rule: self.rule.clone() })
                }
            }
        }
    }

    /// Checks if an event is ready.
    /// Returns true if the subscription is ready
    /// Returns false if the subscription doesn't exist or it exists and it's
    fn is_ready(&mut self) -> bool {
        self.counter.is_ready()
    }
}

/// Represents a counter for a subscription.
#[derive(Debug, Clone)]
pub struct SubscriptionCounter {
    /// The count of events.
    count: u32,
    /// The interval between the events that should trigger the subscription
    interval: u32,
    // TODO: Add dealay for debouncing between events
}

impl Default for SubscriptionCounter {
    fn default() -> Self {
        Self { count: 0, interval: 1 }
    }
}

impl Default for Subscriptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Subscriptions {
    /// Creates a new collection of subscriptions.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Checks if an event is ready and subscribed.
    pub fn is_subscribed_and_ready(&mut self, event: &Event) -> Result<bool, SubscriptionsError> {
        // Create a context mapping from the event
        let context = self.create_context_from_event(event).map_err(|e| {
            SubscriptionsError::ContextCreation { source: e, event: event.to_owned() }
        })?;
        // Evaluate every subscription of the task against the event by using the generated context
        for sub in &mut self.0 {
            if sub.is_subscribed(&context)? && sub.is_ready() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Creates a context from an event. It adds the following elements into the context, so they
    /// can be used during the eval.
    /// - Adds the body of the event
    /// - Adds the origin and type of the event
    /// - Adds the hash function, which hashes a string into an number
    pub fn create_context_from_event(
        &self,
        event: &Event,
    ) -> Result<HashMapContext, EvalexprError> {
        let mut context: HashMapContext = Default::default();
        self.add_body_to_context(event, &mut context)?;
        self.add_origin_and_type_to_context(event, &mut context)?;
        self.add_hash_function_to_context(&mut context)?;
        trace!(context = ?context, "Evaluation Context");
        Ok(context)
    }

    /// Adds the body of the event to the context. It iterates through
    /// all the attributes of the body and adds them as top-level items
    /// in the context.
    ///
    /// ## Example
    /// An event produced by `set_working_fg` in [`crate::tasks::task`], will be evaluated with the
    /// following context: ```
    /// HashMapContext { variables: {"type": String("activity_state_change"), "origin":
    /// Int(1203646832), "background_working": Boolean(false), "foreground_working": Boolean(false)}
    /// ````
    pub fn add_body_to_context(
        &self,
        event: &Event,
        context: &mut HashMapContext,
    ) -> Result<(), EvalexprError> {
        if *event.get_body_ref() != serde_json::Value::Null {
            let body = event.get_body_ref();
            add_value_to_context("body", body, context)?;
        }
        Ok(())
    }

    /// Adds the origin and type of the event to the context.
    fn add_origin_and_type_to_context(
        &self,
        event: &Event,
        context: &mut HashMapContext,
    ) -> Result<(), EvalexprError> {
        context.set_value("origin".to_owned(), (event.get_origin() as i64).into())?;
        context.set_value("type".to_owned(), event.get_type().to_string().into())?;
        Ok(())
    }

    /// Adds the hash function to the context.
    fn add_hash_function_to_context(
        &self,
        context: &mut HashMapContext,
    ) -> Result<(), EvalexprError> {
        context.set_function(
            "hash".to_owned(),
            Function::new(|name| {
                Ok(evalexpr::Value::Int(hash_string(&name.as_string().unwrap()) as i64))
            }),
        )?;
        Ok(())
    }

    /// Adds a new subscription.
    pub fn add(&mut self, subscription: Subscription) {
        self.0.push(subscription)
    }

    pub fn try_from_iterator<I>(iter: I) -> eyre::Result<Self>
    where
        I: IntoIterator<Item = Trigger>,
    {
        let mut subscriptions = Subscriptions::new();
        for trigger in iter {
            let subscription = Subscription::try_from(trigger)?;
            subscriptions.add(subscription);
        }

        Ok(subscriptions)
    }
}

impl SubscriptionCounter {
    /// Checks if the counter is ready.
    fn is_ready(&mut self) -> bool {
        self.increment();
        self.count % self.interval == 0
    }

    fn increment(&mut self) {
        self.count += 1;
    }

    fn new(interval: u32) -> SubscriptionCounter {
        Self { interval, count: 0 }
    }
}

impl From<u32> for SubscriptionCounter {
    fn from(interval: u32) -> Self {
        Self::new(interval)
    }
}

impl TryFrom<Trigger> for Subscription {
    type Error = eyre::Report;
    fn try_from(trigger: Trigger) -> eyre::Result<Self> {
        let counter = SubscriptionCounter::new(trigger.interval);
        let rule = trigger.rule;
        Ok(Self::new(rule, counter))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SubscriptionsError {
    #[error("Evaluation failed for rule [ {rule} ]")]
    Eval { source: EvalexprError, rule: String },
    #[error("Subscription failed to create context for event {event}")]
    ContextCreation { source: EvalexprError, event: Event },
}

/// Adds a value to the context. If the value is an object or an array,
/// it recursively adds its children to the context.
fn add_value_to_context(
    prefix: &str,
    value: &serde_json::Value,
    context: &mut HashMapContext,
) -> Result<(), EvalexprError> {
    match value {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                let new_key = if prefix.is_empty() && !key.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                };
                add_value_to_context(&new_key, value, context)?;
            }
        }
        serde_json::Value::Array(arr) => {
            for (index, value) in arr.iter().enumerate() {
                let new_key = format!("{}[{}]", prefix, index);
                add_value_to_context(&new_key, value, context)?;
            }
        }
        serde_json::Value::Number(ref n) => {
            let evalexpr_value = n
                .as_i64()
                .map(evalexpr::Value::Int)
                .or_else(|| n.as_f64().map(evalexpr::Value::Float))
                .unwrap();
            context.set_value(prefix.to_owned(), evalexpr_value)?;
        }
        serde_json::Value::String(ref s) => {
            context.set_value(prefix.to_owned(), evalexpr::Value::String(s.clone()))?;
        }
        serde_json::Value::Bool(ref b) => {
            context.set_value(prefix.to_owned(), evalexpr::Value::Boolean(*b))?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use crate::events::{EventBuilder, EventType};

    use super::*;
    use evalexpr::Value;
    use phylax_monitor::{evm_alert::alert::AlertState, EvmAlertResult};

    #[test]
    fn test_subscription_is_subscribed() {
        let mut context: HashMapContext = Default::default();
        context.set_value("origin".to_owned(), Value::Int(1)).unwrap();
        context.set_value("type".to_owned(), Value::String("test".to_string())).unwrap();

        let mut subscription = Subscription::new(
            "origin == 1 && type == \"test\"".to_string(),
            SubscriptionCounter::new(1),
        );
        assert!(subscription.is_subscribed(&context).unwrap());

        let mut subscription = Subscription::new(
            "origin == 2 && type == \"test\"".to_string(),
            SubscriptionCounter::new(1),
        );
        assert!(!subscription.is_subscribed(&context).unwrap());
    }

    #[test]
    fn test_subscription_counter_is_ready() {
        let mut counter = SubscriptionCounter::new(1);
        assert!(counter.is_ready());

        let mut counter = SubscriptionCounter::new(2);
        assert!(!counter.is_ready());
        assert!(counter.is_ready());
    }

    #[test]
    fn test_subscriptions_is_subscribed_and_ready() {
        let mut subscriptions = Subscriptions::new();
        let subscription = Subscription::new(
            "origin == 1 && type == \"activate_task\"".to_string(),
            SubscriptionCounter::new(1),
        );
        subscriptions.add(subscription);

        let event = EventBuilder::new().with_origin(1).with_type(EventType::ActivateTask).build();
        match subscriptions.is_subscribed_and_ready(&event) {
            Ok(result) => assert!(result),
            Err(_) => panic!("Failed to check if event is subscribed and ready"),
        }

        let event =
            EventBuilder::new().with_origin(2).with_type(EventType::ActionExecution).build();
        match subscriptions.is_subscribed_and_ready(&event) {
            Ok(result) => assert!(!result),
            Err(_) => panic!("Failed to check if event is subscribed and ready"),
        }
    }
    #[test]
    fn test_subscription_complex_rule() {
        let mut context: HashMapContext = Default::default();
        context.set_value("origin".to_owned(), Value::Int(1)).unwrap();
        context.set_value("type".to_owned(), Value::String("test".to_string())).unwrap();
        context.set_value("extra".to_owned(), Value::String("extra".to_string())).unwrap();

        let mut subscription = Subscription::new(
            "origin == 1 && type == \"test\" && extra == \"extra\"".to_string(),
            SubscriptionCounter::new(1),
        );
        assert!(subscription.is_subscribed(&context).unwrap());
    }
    #[test]
    fn test_subscription_counter_large_interval() {
        let mut counter = SubscriptionCounter::new(1000);
        for _ in 0..999 {
            assert!(!counter.is_ready());
        }
        assert!(counter.is_ready());
    }

    #[test]
    fn test_subscriptions_multiple_subscriptions() {
        let mut subscriptions = Subscriptions::new();
        let subscription1 = Subscription::new(
            "origin == 1 && type == \"activate_task\"".to_string(),
            SubscriptionCounter::new(1),
        );
        let subscription2 = Subscription::new(
            "origin == 2 && type == \"test\"".to_string(),
            SubscriptionCounter::new(1),
        );
        subscriptions.add(subscription1);
        subscriptions.add(subscription2);

        let event1 = EventBuilder::new().with_origin(1).with_type(EventType::ActivateTask).build();
        match subscriptions.is_subscribed_and_ready(&event1) {
            Ok(result) => assert!(result),
            Err(_) => panic!("Failed to check if event1 is subscribed and ready"),
        }

        let event2 = EventBuilder::new().with_origin(2).with_type(EventType::ConfigChanged).build();
        match subscriptions.is_subscribed_and_ready(&event2) {
            Ok(result) => assert!(!result),
            Err(_) => panic!("Failed to check if event2 is subscribed and ready"),
        }
    }

    #[test]
    fn test_subscriptions_no_subscriptions() {
        let mut subscriptions = Subscriptions::new();
        let event = EventBuilder::new().with_origin(1).with_type(EventType::ApiCommand).build();
        match subscriptions.is_subscribed_and_ready(&event) {
            Ok(result) => assert!(!result),
            Err(_) => panic!("Failed to check if event is subscribed and ready"),
        }
    }

    #[test]
    fn test_add_value_to_context_complex_json() {
        let mut context: HashMapContext = Default::default();
        let value = serde_json::json!({
            "key1": "value1",
            "key2": {
                "subkey1": "subvalue1",
                "subkey2": [1, 2, 3]
            }
        });
        add_value_to_context("", &value, &mut context).unwrap();
        assert_eq!(context.get_value("key1").unwrap(), &Value::String("value1".to_string()));
        assert_eq!(
            context.get_value("key2.subkey1").unwrap(),
            &Value::String("subvalue1".to_string())
        );
        assert_eq!(context.get_value("key2.subkey2[0]").unwrap(), &Value::Int(1));
        assert_eq!(context.get_value("key2.subkey2[1]").unwrap(), &Value::Int(2));
        assert_eq!(context.get_value("key2.subkey2[2]").unwrap(), &Value::Int(3));
    }

    #[test]
    fn test_subscriptions_duplicate_subscriptions() {
        let mut subscriptions = Subscriptions::new();
        let subscription = Subscription::new(
            "origin == 1 && type == \"alert_firing\"".to_string(),
            SubscriptionCounter::new(1),
        );
        subscriptions.add(subscription.clone());
        subscriptions.add(subscription);

        let event = EventBuilder::new()
            .with_origin(1)
            .with_type(EventType::AlertFiring)
            .with_body(serde_json::Value::Null)
            .build();
        assert!(subscriptions.is_subscribed_and_ready(&event).unwrap());
    }

    #[test]
    fn test_subscriptions_with_event_bodies() {
        let mut subscriptions = Subscriptions::new();
        let subscription = Subscription::new(
            "origin == 1 && body.alert_state == \"Firing\"".to_string(),
            SubscriptionCounter::new(1),
        );
        subscriptions.add(subscription.clone());
        subscriptions.add(subscription);
        let failed = HashMap::new();
        let body = EvmAlertResult {
            alert_state: AlertState::Firing,
            failed_tests: failed,
            failed_setup: false,
            duration: Duration::from_secs(1),
            all_exposed_values: vec![],
        };
        let event = EventBuilder::new()
            .with_origin(1)
            .with_type(EventType::AlertFiring)
            .with_body(serde_json::to_value(body).unwrap())
            .build();
        assert!(subscriptions.is_subscribed_and_ready(&event).unwrap());
    }

    #[test]
    fn test_add_value_to_context_null_value() {
        let mut context: HashMapContext = Default::default();
        let value = serde_json::json!(null);
        add_value_to_context("key", &value, &mut context).unwrap();
        assert!(context.get_value("key").is_none());
    }
}
