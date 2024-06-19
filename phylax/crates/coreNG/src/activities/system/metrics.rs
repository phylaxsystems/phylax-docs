use async_trait::async_trait;
use eyre::Result;
use flume::r#async::RecvStream;
use phylax_common::metrics::*;
use sisyphus_tasks::metrics::MetricHandle;
use std::{collections::HashMap, fmt::Display, option::Option, time::Instant};

use crate::{
    activities::{Activity, FgWorkType},
    events::Event,
    tasks::task::WorkContext,
};

/// Struct for managing metrics.
pub struct Metrics {
    /// A map from task ID to the last time an event was received from that task.
    task_last_event: HashMap<u32, Instant>,
}

/// Default implementation for Metrics.
impl Default for Metrics {
    /// Returns a new Metrics instance.
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    /// Creates a new Metrics instance.
    pub fn new() -> Self {
        Self { task_last_event: Default::default() }
    }

    /// Updates the events per second metric for a given task.
    fn update_events_per_second(
        &mut self,
        event: Event,
        task_name: &str,
        metrics: &MetricsRegistry,
    ) {
        let last_time =
            self.task_last_event.get(&event.origin).unwrap_or(&Instant::now()).to_owned();
        self.task_last_event.insert(event.origin, Instant::now());
        let now = Instant::now();
        let duration = now.duration_since(last_time).as_secs_f64();
        let metric = metrics.inner().gv(EVENT_BUS_PER_SECOND).metric([task_name]);
        metric.set(1.0 / duration);
    }

    /// Updates the total events metric for a given task.
    fn update_total_events(&self, task_name: &str, metrics: &MetricsRegistry) {
        let metric = metrics.inner().cv(EVENT_BUS_TOTAL).metric([task_name]);
        metric.inc();
    }
}

/// Display implementation for Metrics.
impl Display for Metrics {
    /// Formats the Metrics for display.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Phylax Metrics Task")
    }
}

/// Activity implementation for Metrics.
#[async_trait]
impl Activity for Metrics {
    type Command = MetricCmd;
    type InternalStream = RecvStream<'static, Self::InternalStreamItem>;
    type InternalStreamItem = ();

    /// Returns the work type of the Metrics activity.
    fn work_type(&self) -> FgWorkType {
        FgWorkType::NonBlocking
    }

    /// Decides what commands to execute based on the given events.
    fn decide(&self, events: Vec<&Event>, _context: &WorkContext) -> Option<Vec<MetricCmd>> {
        Some(vec![MetricCmd::CountEvent(events[0].clone())])
    }

    /// Executes the given commands. It runs on every event.
    async fn do_work(&mut self, cmds: Vec<MetricCmd>, ctx: WorkContext) -> Result<()> {
        for cmd in cmds {
            match cmd {
                MetricCmd::CountEvent(event) => {
                    let task_name = ctx
                        .activity
                        .dns
                        .find_name_from_id(event.origin)
                        .unwrap_or("Unknown Task".to_string());
                    self.update_events_per_second(event, &task_name, &ctx.activity.metrics);
                    self.update_total_events(&task_name, &ctx.activity.metrics);
                }
            }
        }
        Ok(())
    }
}

/// Enum for different types of metric commands.
#[derive(Debug, PartialEq)]
pub enum MetricCmd {
    /// Command to count an event.
    CountEvent(Event),
}

/// Display implementation for MetricCmd.
impl Display for MetricCmd {
    /// Formats the MetricCmd for display.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricCmd::CountEvent(event) => write!(f, "CountEvent({})", event),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{events::EventBuilder, mocks::mock_context::mock_activity_context};

    use super::*;
    use phylax_common::metrics::MetricsRegistry;
    use std::{thread::sleep, time::Duration};

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        assert_eq!(metrics.task_last_event.len(), 0);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = Metrics::default();
        assert_eq!(metrics.task_last_event.len(), 0);
    }

    #[test]
    fn test_update_events_per_second() {
        let mut metrics = Metrics::new();
        let metrics_registry = MetricsRegistry::new();
        let event = Event { origin: 1, ..Default::default() };

        metrics.update_events_per_second(event.clone(), "test_task", &metrics_registry);
        sleep(Duration::from_secs(1));
        metrics.update_events_per_second(event, "test_task", &metrics_registry);

        let metric = metrics_registry.inner().gv(EVENT_BUS_PER_SECOND).metric(["test_task"]);
        assert!(metric.get() > 0.0);
    }

    #[test]
    fn test_update_total_events() {
        let metrics = Metrics::new();
        let metrics_registry = MetricsRegistry::new();

        metrics.update_total_events("test_task", &metrics_registry);
        let metric = metrics_registry.inner().cv(EVENT_BUS_TOTAL).metric(["test_task"]);
        assert_eq!(metric.get(), 1_f64);
    }

    #[test]
    fn test_display() {
        let metrics = Metrics::new();
        assert_eq!(format!("{}", metrics), "Phylax Metrics Task");
    }

    #[tokio::test]
    async fn test_work_type() {
        let metrics = Metrics::new();
        assert_eq!(metrics.work_type(), FgWorkType::NonBlocking);
    }

    #[tokio::test]
    async fn test_decide() {
        let metrics = Metrics::new();
        let event = Event { origin: 1, ..Default::default() };
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let result = metrics.decide(vec![&event], &context);
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_do_work() {
        let mut metrics = Metrics::new();
        let event = Event { origin: 1, ..Default::default() };
        let cmd = MetricCmd::CountEvent(event);
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let result = metrics.do_work(vec![cmd], context).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_metric_cmd_display() {
        let event = Event { origin: 1, ..Default::default() };
        let cmd = MetricCmd::CountEvent(event);
        assert_eq!(
            format!("{}", cmd),
            "CountEvent(EVENT-[ Origin:\"1\" | Type:\"info\" | B:<..> ])"
        );
    }

    #[tokio::test]
    async fn test_do_work_no_cmds() {
        let mut metrics = Metrics::new();
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let result = metrics.do_work(vec![], context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_decide_different_events() {
        let metrics = Metrics::new();
        let event1 = Event { origin: 1, ..Default::default() };
        let event2 = Event { origin: 2, ..Default::default() };
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());

        let result1 = metrics.decide(vec![&event1], &context);
        assert_eq!(result1, Some(vec![MetricCmd::CountEvent(event1.clone())]));

        let result2 = metrics.decide(vec![&event2], &context);
        assert_eq!(result2, Some(vec![MetricCmd::CountEvent(event2.clone())]));
    }
}
