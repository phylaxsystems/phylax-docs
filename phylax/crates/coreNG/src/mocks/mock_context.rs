use phylax_common::metrics::MetricsRegistry;

use crate::{activities::ActivityContext, events::EventBus, tasks::dns::TaskDns};
use std::sync::Arc;

/// Creates a mock ActivityContext for testing.
#[allow(dead_code)]
pub fn mock_activity_context(name: Option<String>) -> Arc<ActivityContext> {
    let task_name = name.unwrap_or("mock_task".to_owned());
    let metrics = MetricsRegistry::new();
    let dns = Arc::new(TaskDns::new());
    let bus = EventBus::new();
    let (shutdown_sender, _) = tokio::sync::broadcast::channel(1);
    Arc::new(ActivityContext::new(task_name, metrics, dns, bus, shutdown_sender))
}
