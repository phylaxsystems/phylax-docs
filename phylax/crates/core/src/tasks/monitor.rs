use std::collections::HashMap;

use super::metric::NumericMetric;
use futures::{
    stream::{select_all, SelectAll},
    StreamExt,
};
use opentelemetry::{self, global};
use phylax_interfaces::{
    activity::{Activity, MonitorConfig, PointValue},
    context::{ActivityContext, HealthData},
    error::PhylaxError,
    message::MessageDetails,
};
use phylax_tasks::monitors::MonitorReceiver;
use phylax_tracing::{
    metrics::NODE_METER_NAME,
    tracing::{error, warn},
};
use tokio::select;

/// Watches for activity and health monitor outputs and records them to metrics, and outputs
/// relevant notification messages. By default, if no activity or health monitor outputs are
/// received, the task will sleep for 200ms.
pub(crate) struct MonitorWatcher {
    /// Receiver stream for activity monitor outputs.
    pub activity_monitor_receivers: SelectAll<MonitorReceiver<PointValue>>,
    /// Receiver stream for health monitor outputs.
    pub health_monitor_receivers: SelectAll<MonitorReceiver<HealthData>>,
    /// Maps monitor names to metrics.
    pub metrics: HashMap<&'static str, NumericMetric>,
}

impl MonitorWatcher {
    /// The default sleep duration in milliseconds when no activity or health monitor outputs are
    /// received.
    const SLEEP_DURATION_IN_MS: u64 = 200;

    /// Create a new [`MonitorWatcher`] from activity and health monitor receivers and monitor
    /// configs.
    pub fn new(
        activity_monitor_receivers: Vec<MonitorReceiver<PointValue>>,
        health_monitor_receivers: Vec<MonitorReceiver<HealthData>>,
        monitor_configs: &[&MonitorConfig],
    ) -> Self {
        let mut metrics = HashMap::new();
        let meter = global::meter(NODE_METER_NAME);

        // Build metrics from monitor configs
        for monitor_config in monitor_configs {
            let maybe_metric: Option<NumericMetric> =
                NumericMetric::from_config(monitor_config, &meter);
            if let Some(metric) = maybe_metric {
                metrics.insert(monitor_config.name, metric);
            }
        }

        Self {
            activity_monitor_receivers: select_all(activity_monitor_receivers),
            health_monitor_receivers: select_all(health_monitor_receivers),
            metrics,
        }
    }
}

// TODO: Replace with real notification message.
pub struct DummyNotificationMessage;

impl Activity for MonitorWatcher {
    const FLAVOR_NAME: &'static str = "monitor_watcher_flavor";
    type Input = ();
    type Output = DummyNotificationMessage;

    /// Processes incoming activity and health monitor outputs and records them to metrics, and
    /// additionally outputs aggregated notification messages.
    async fn process_message(
        &mut self,
        _message: Option<(Self::Input, MessageDetails)>,
        _context: ActivityContext,
    ) -> Result<Option<Self::Output>, PhylaxError> {
        select! {
            Some(activity_data_output) = &mut self.activity_monitor_receivers.next() => {
                let monitor_name = activity_data_output.monitor_name;
                let maybe_metric = self.metrics.get(monitor_name);
                match (maybe_metric, activity_data_output.data) {
                    (Some(NumericMetric::Float(f64_metric)), PointValue::Float(value)) => {
                        f64_metric.record(value, &[]);
                    }
                    (Some(NumericMetric::Int(i64_metric)), PointValue::Int(value)) => {
                        i64_metric.record(value, &[]);
                    }
                    (Some(NumericMetric::Uint(u64_metric)), PointValue::Uint(value)) => {
                        u64_metric.record(value, &[]);
                    }
                    (Some(metric), point_value) => {
                        warn!(monitor_name, ?point_value, metric_type=metric.to_string(), "Metric received an incorrect data type");
                    },
                    (None, _) => {
                        error!(monitor_name, "Received data for metric that was not registered");
                    }
                }
                Ok(None)
            },
            Some(health_data_output) = &mut self.health_monitor_receivers.next() => {
                // TODO: Add notification message, aggregated by task.
                let monitor_name = health_data_output.monitor_name;
                let maybe_metric = self.metrics.get(monitor_name);
                match (maybe_metric, health_data_output.data.value) {
                    (Some(NumericMetric::Float(f64_metric)), PointValue::Float(value)) => {
                        f64_metric.record(value, &[]);
                    }
                    (Some(NumericMetric::Int(i64_metric)), PointValue::Int(value)) => {
                        i64_metric.record(value, &[]);
                    }
                    (Some(NumericMetric::Uint(u64_metric)), PointValue::Uint(value)) => {
                        u64_metric.record(value, &[]);
                    }
                    (Some(metric), point_value) => {
                        warn!(monitor_name, ?point_value, metric_type=metric.to_string(), "Metric received an incorrect data type");
                    },
                    (None, _) => {
                        error!(monitor_name, "Sent a point data to a health metric that was not registered");
                    }
                }
                Ok(None)
            },
            else => {
                // When there are no incoming metrics, sleep to avoid busy looping
                tokio::time::sleep(tokio::time::Duration::from_millis(Self::SLEEP_DURATION_IN_MS)).await;
                Ok(None)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phylax_interfaces::activity::{MetricKind, MonitorKind};

    #[test]
    fn test_monitor_watcher_configures_only_non_null_metrics() {
        let monitor_configs = &[&MonitorConfig {
            name: "float_metric",
            description: "A valid test float metric",
            unit: "unit",
            display: None,
            metric_type: Some(MetricKind::Histogram),
            value_type: PointValue::Float(0.0),
            category: MonitorKind::Health { unhealthy_message: "I'm unhealthy!" },
        },
        &MonitorConfig {
            name: "theoretically_invalid_metric",
            description: "A test monitor with an associated non-null metric that is technically invalid in the OTel SDK",
            unit: "unit",
            display: None,
            metric_type: Some(MetricKind::Counter),
            value_type: PointValue::Uint(0),
            category: MonitorKind::Default,
        },
        &MonitorConfig {
            name: "monitor_no_metric",
            description: "A test monitor with no associated metric",
            unit: "unit",
            display: None,
            metric_type: None,
            value_type: PointValue::Int(0),
            category: MonitorKind::Default,
        }];

        let monitor_watcher = MonitorWatcher::new(vec![], vec![], monitor_configs);

        assert_eq!(monitor_watcher.metrics.len(), 2);
        assert!(monitor_watcher.metrics.contains_key("float_metric"));
        assert!(monitor_watcher.metrics.contains_key("theoretically_invalid_metric"));
    }
}
