use opentelemetry::Context as OtelContext;
use phylax_interfaces::{
    activity::{MonitorConfig, MonitorKind, PointValue},
    context::{ActivityMonitor, HealthData, HealthMonitor},
};
use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc;
use tokio_stream::Stream;

use crate::task::TaskDetails;

/// A struct representing a receiver for receiving [`PointValue`] or [`HealthData`] data, which
/// returns those as an async stream of wrapped [`MonitorData`].
pub struct MonitorReceiver<T> {
    /// The name of the monitor.
    pub monitor_name: &'static str,
    /// The details of the task that the monitor belongs to.
    pub task_details: TaskDetails,
    /// The OpenTelemetry context of the task that the monitor belongs to.
    pub task_otel_context: OtelContext,
    /// The receiver for the monitor's data.
    pub receiver: mpsc::UnboundedReceiver<T>,
}

/// A struct representing the decorated output data from a `MonitorSender`.
pub struct MonitorData<T> {
    /// The name of the monitor.
    pub monitor_name: &'static str,
    /// The details of the task that the monitor belongs to.
    pub task_details: TaskDetails,
    /// The OpenTelemetry context of the task that the monitor belongs to.
    pub task_otel_context: OtelContext,
    /// The data received from the monitor.
    pub data: T,
}

impl<T> Stream for MonitorReceiver<T> {
    type Item = MonitorData<T>;

    /// Decorate any data received by a monitor with the monitor's name, task details, and
    /// OpenTelemetry context.
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.receiver).poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                let data = MonitorData {
                    monitor_name: this.monitor_name,
                    task_details: this.task_details.clone(),
                    task_otel_context: this.task_otel_context.clone(),
                    data,
                };
                Poll::Ready(Some(data))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Struct representing the channels used for sending data to activity and health monitors.
pub struct MonitorChannels {
    pub activity_senders: HashMap<&'static str, ActivityMonitor>,
    pub activity_receivers: Vec<MonitorReceiver<PointValue>>,
    pub health_senders: HashMap<&'static str, HealthMonitor>,
    pub health_receivers: Vec<MonitorReceiver<HealthData>>,
}

/// Constructs and returns  `MonitorChannels` struct, a container for channels receivers and senders
/// for individual monitors of an [`Activity`], by iterating over the provided `monitor_configs`
/// slice. Depending on the `MonitorCategory` of each `MonitorConfig`, it creates and assigns the
/// appropriate senders and receivers for both activity and health monitors.
pub fn build_monitor_channels(
    monitor_configs: &[MonitorConfig],
    task_details: &TaskDetails,
    task_otel_context: &OtelContext,
) -> MonitorChannels {
    let mut activity_senders = HashMap::new();
    let mut activity_receivers = Vec::new();
    let mut health_senders = HashMap::new();
    let mut health_receivers = Vec::new();
    for monitor_config in monitor_configs {
        if let MonitorKind::Health { .. } = monitor_config.category {
            let (health_tx, health_rx) = mpsc::unbounded_channel::<HealthData>();
            let sender = HealthMonitor::new(monitor_config.name, health_tx);
            let receiver = MonitorReceiver::<HealthData> {
                monitor_name: monitor_config.name,
                task_details: task_details.clone(),
                task_otel_context: task_otel_context.clone(),
                receiver: health_rx,
            };
            health_senders.insert(monitor_config.name, sender);
            health_receivers.push(receiver);
        } else {
            let (activity_tx, activity_rx) = mpsc::unbounded_channel::<PointValue>();
            let sender = ActivityMonitor::new(monitor_config.name, activity_tx);
            let receiver = MonitorReceiver::<PointValue> {
                monitor_name: monitor_config.name,
                task_details: task_details.clone(),
                task_otel_context: task_otel_context.clone(),
                receiver: activity_rx,
            };
            activity_senders.insert(monitor_config.name, sender);
            activity_receivers.push(receiver);
        }
    }
    MonitorChannels { activity_senders, activity_receivers, health_senders, health_receivers }
}

#[cfg(test)]
mod tests {
    use phylax_interfaces::activity::MonitorKind;

    use super::*;
    use crate::{monitors::MonitorConfig, task::ActivityCategory};

    fn build_dummy_monitor_channels(monitor_configs: &[MonitorConfig]) -> MonitorChannels {
        let task_details = TaskDetails {
            task_name: "task_name".to_string(),
            task_flavor: "task_flavor".to_string(),
            task_labels: &[],
            task_category: ActivityCategory::Watcher,
        };
        let task_otel_context = OtelContext::new();
        build_monitor_channels(monitor_configs, &task_details, &task_otel_context)
    }

    #[test]
    fn test_build_monitor_channels_with_health_monitors() {
        let monitor_configs = vec![
            MonitorConfig {
                name: "health_monitor_1",
                description: "Health monitor 1 - it monitors health",
                unit: "very real units",
                category: MonitorKind::Health { unhealthy_message: "unhealthy" },
                metric_type: None,
                value_type: PointValue::Float(0.0),
                display: None,
            },
            MonitorConfig {
                name: "health_monitor_2",
                description: "Health monitor 2 - it monitors other health",
                unit: "very real units",
                category: MonitorKind::Health { unhealthy_message: "unhealthy" },
                metric_type: None,
                value_type: PointValue::Float(0.0),
                display: None,
            },
        ];

        let monitor_channels = build_dummy_monitor_channels(&monitor_configs);

        assert_eq!(monitor_channels.health_senders.len(), 2);
        assert_eq!(monitor_channels.health_receivers.len(), 2);
        assert_eq!(monitor_channels.activity_senders.len(), 0);
        assert_eq!(monitor_channels.activity_receivers.len(), 0);

        assert!(monitor_channels.health_senders.contains_key("health_monitor_1"));
        assert!(monitor_channels.health_senders.contains_key("health_monitor_2"));
        assert_eq!(monitor_channels.health_receivers[0].monitor_name, "health_monitor_1");
        assert_eq!(monitor_channels.health_receivers[1].monitor_name, "health_monitor_2");
    }

    #[test]
    fn test_build_monitor_channels_with_activity_monitors() {
        let monitor_configs = vec![
            MonitorConfig {
                name: "activity_monitor_1",
                description: "Activity monitor 1 - it monitors activity",
                unit: "very real units",
                category: MonitorKind::Default,
                metric_type: None,
                value_type: PointValue::Float(0.0),
                display: None,
            },
            MonitorConfig {
                name: "activity_monitor_2",
                description: "Activity monitor 2 - it monitors other activity",
                unit: "very real units",
                category: MonitorKind::Default,
                metric_type: None,
                value_type: PointValue::Float(0.0),
                display: None,
            },
        ];

        let monitor_channels = build_dummy_monitor_channels(&monitor_configs);

        assert_eq!(monitor_channels.health_senders.len(), 0);
        assert_eq!(monitor_channels.health_receivers.len(), 0);
        assert_eq!(monitor_channels.activity_senders.len(), 2);
        assert_eq!(monitor_channels.activity_receivers.len(), 2);

        assert!(monitor_channels.activity_senders.contains_key("activity_monitor_1"));
        assert!(monitor_channels.activity_senders.contains_key("activity_monitor_2"));
        assert_eq!(monitor_channels.activity_receivers[0].monitor_name, "activity_monitor_1");
        assert_eq!(monitor_channels.activity_receivers[1].monitor_name, "activity_monitor_2");
    }

    #[test]
    fn test_build_monitor_channels_with_mixed_monitors() {
        let monitor_configs = vec![
            MonitorConfig {
                name: "health_monitor_1",
                description: "Health monitor 1 - it monitors health",
                unit: "very real units",
                category: MonitorKind::Health { unhealthy_message: "unhealthy" },
                metric_type: None,
                value_type: PointValue::Float(0.0),
                display: None,
            },
            MonitorConfig {
                name: "activity_monitor_1",
                description: "Activity monitor 1 - it monitors activity",
                unit: "very real units",
                category: MonitorKind::Default,
                metric_type: None,
                value_type: PointValue::Float(0.0),
                display: None,
            },
        ];

        let monitor_channels = build_dummy_monitor_channels(&monitor_configs);

        assert_eq!(monitor_channels.health_senders.len(), 1);
        assert_eq!(monitor_channels.health_receivers.len(), 1);
        assert_eq!(monitor_channels.activity_senders.len(), 1);
        assert_eq!(monitor_channels.activity_receivers.len(), 1);

        assert!(monitor_channels.health_senders.contains_key("health_monitor_1"));
        assert_eq!(monitor_channels.health_receivers[0].monitor_name, "health_monitor_1");
        assert!(monitor_channels.activity_senders.contains_key("activity_monitor_1"));
        assert_eq!(monitor_channels.activity_receivers[0].monitor_name, "activity_monitor_1");
    }
}
