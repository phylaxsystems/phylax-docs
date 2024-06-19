use crate::Trigger;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsConfig {
    pub on: Vec<Trigger>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { on: default_triggers() }
    }
}

/// The metrics task is subscribed to all events
fn default_triggers() -> Vec<Trigger> {
    let trigger = Trigger { rule: "origin > 0".to_owned(), interval: 1 };
    vec![trigger]
}
