use crate::{activity::PointValue, event::EventBus, state_registry::StateRegistry};
use reth_tasks::TaskExecutor;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;

/// Context provider struct for all [`Activity`] trait functions.
pub struct ActivityContext {
    /// An executor for spawning async tasks whose failures can signal to the node that
    /// it is necessary to shutdown.
    pub executor: TaskExecutor,
    /// A handle to the global event bus for sending or receiving events to and from the rest of
    /// the Phylax node or Event API.
    pub event_bus: EventBus,
    /// A map of senders by the name specified in for default activity monitor data, useful for
    /// exporting data.
    pub activity_monitors: HashMap<&'static str, ActivityMonitor>,
    /// A map of senders for health monitor data, useful for tracking health and initiating
    /// warning notifications in addition to the functionality of normal activity monitor.
    pub health_monitors: HashMap<&'static str, HealthMonitor>,
    /// Type Demarcated Shared Global state
    pub state_registry: Arc<StateRegistry>,
}

/// A health datapoint to be added to a [`MonitorCategory::Health`] monitor's dataset, including a
/// severity level.
#[derive(Clone, Debug)]
pub struct HealthData {
    /// The numerical value of the datapoint.
    pub value: PointValue,
    /// The severity of the health status.
    pub severity: HealthSeverity,
}

/// A helper struct for sending data to a [`MonitorCategory::Default`] monitor if one was configured
/// in the [`Activity`] `MONITORS` constant.
#[derive(Clone, Debug)]
pub struct ActivityMonitor {
    /// The name of the activity monitor.
    pub name: &'static str,
    /// The unbounded sender for activity data.
    pub(crate) sender: mpsc::UnboundedSender<PointValue>,
}

/// A helper struct for sending data to a [`MonitorCategory::Health`] monitor if one was configured
/// in the [`Activity`] `MONITORS` constant.
///
/// # Warnings and errors
/// If the inner MonitorDatapoint If the inner type of the data point cannot be soundly converted to
/// the data type configured in the monitor, it will emit an error log.
#[derive(Clone, Debug)]
pub struct HealthMonitor {
    /// The name of the health monitor.
    pub name: &'static str,
    /// The unbounded sender for health data.
    pub(crate) sender: mpsc::UnboundedSender<HealthData>,
}

impl ActivityMonitor {
    pub fn new(name: &'static str, sender: mpsc::UnboundedSender<PointValue>) -> Self {
        Self { name, sender }
    }

    /// Send a [`MonitorDatapoint`] to the activity monitor dataset.
    pub fn send(&self, datapoint: PointValue) {
        let _ = self.sender.send(datapoint);
    }

    /// Send a i64 to be convered to a [`MonitorDatapoint`] and to be added to this monitor dataset.
    pub fn send_i64(&self, value: i64) {
        let _ = self.sender.send(PointValue::Int(value));
    }

    /// Send a u64 to be convered to a [`MonitorDatapoint`] and to be added to this monitor dataset.
    pub fn send_u64(&self, value: u64) {
        let _ = self.sender.send(PointValue::Uint(value));
    }

    /// Send a f64 to be convered to a [`MonitorDatapoint`] and to be added to this monitor dataset.
    pub fn send_f64(&self, value: f64) {
        let _ = self.sender.send(PointValue::Float(value));
    }
}

impl HealthMonitor {
    pub fn new(name: &'static str, sender: mpsc::UnboundedSender<HealthData>) -> Self {
        Self { name, sender }
    }

    /// Send a [`HealthData`] datapoint to the health monitor dataset.
    pub fn send(&self, datapoint: HealthData) {
        let _ = self.sender.send(datapoint);
    }

    /// Send a i64 and severity to be convered to a [`HealthData`] datapoint and to be added to this
    /// health monitor dataset.
    pub fn send_i64(&self, value: i64, severity: HealthSeverity) {
        let _ = self.sender.send(HealthData { value: PointValue::Int(value), severity });
    }

    /// Send a f64 and severity to be convered to a [`HealthData`] datapoint and to be added to this
    /// health monitor dataset.
    pub fn send_u64(&self, value: u64, severity: HealthSeverity) {
        let _ = self.sender.send(HealthData { value: PointValue::Uint(value), severity });
    }

    /// Send a f64 and severity to be convered to a [`HealthData`] datapoint and to be added to this
    /// health monitor dataset.
    pub fn send_f64(&self, value: f64, severity: HealthSeverity) {
        let _ = self.sender.send(HealthData { value: PointValue::Float(value), severity });
    }
}

/// Represents the severity level of health data in a [`HealthMonitor`].
#[derive(Clone, Debug)]
pub enum HealthSeverity {
    /// Indicates that the health monitor is in a healthy state.
    Healthy,
    /// Indicates that the health monitor has degraded into an unhealthy health and should emit a
    /// warning message.
    Warning,
    /// Indicates that the health monitor is in a critical state and should emit a warning message.
    Critical,
}
