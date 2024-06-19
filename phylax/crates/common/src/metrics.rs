use eyre::Result;
use phylax_tracing::tracing::debug;
use sisyphus_tasks::metrics::*;
use std::sync::Arc;

/// Network lag between block creation and watcher detection in seconds.
const WATCHER_NETWORK_LAG: IntGaugeVecDescriptor<1> = IntGaugeVecDescriptor::new(
    Some("task:watcher"),
    "watcher_network_lag",
    "network_lag_between_block_creation_and_watcher_detection_in_seconds",
    ["chain"],
);

/// Block number of the last detected block.
const WATCHER_BLOCK_NUMBER: IntGaugeVecDescriptor<1> = IntGaugeVecDescriptor::new(
    Some("task:watcher"),
    "watcher_block_number",
    "block_number_of_last_detected_block",
    ["chain"],
);

/// If 1, then the alert is firing.
const ALERT_FIRING: GaugeVecDescriptor<1> =
    GaugeVecDescriptor::new(Some("task:alert"), "alert_firing", "if_1_then_alert_firing", ["name"]);

/// How many seconds did the alert take to run.
const ALERT_DURATION: GaugeVecDescriptor<1> = GaugeVecDescriptor::new(
    Some("task:alert"),
    "last_alert_duration",
    "how_many_seconds_did_alert_take_to_run",
    ["name"],
);

/// Number of times the task has run.
const TASK_INVOCATIONS: CounterVecDescriptor<1> =
    CounterVecDescriptor::new(Some("task"), "runs", "number_of_times_the_task_has_ran", ["name"]);

/// Number of times a task has errored.
const TASK_ERRORS: CounterVecDescriptor<1> = CounterVecDescriptor::new(
    Some("task"),
    "task_errors",
    "number_of_times_a_task_has_errored",
    ["name"],
);

/// If 1, then the activity is working.
const TASK_WORKING: IntGaugeVecDescriptor<2> = IntGaugeVecDescriptor::new(
    Some("task"),
    "task_working",
    "if_1_then_activity_is_working",
    ["task_name", "work_type"],
);

/// Number of events generated.
pub const EVENT_BUS_TOTAL: CounterVecDescriptor<1> = CounterVecDescriptor::new(
    Some("task:event_bus"),
    "total_events",
    "number_of_events_generated",
    ["task_name"],
);

/// Number of events generated per second.
pub const EVENT_BUS_PER_SECOND: GaugeVecDescriptor<1> = GaugeVecDescriptor::new(
    Some("task:event_bus"),
    "events_per_second",
    "number_of_events_generated_per_second",
    ["task_name"],
);

/// Value of exported data.
pub const EXPORTED_DATA_VALUE: GaugeVecDescriptor<4> = GaugeVecDescriptor::new(
    Some("task:alert"),
    "exported_value",
    "value_of_exported_data",
    ["task_name", "key", "chain", "context"],
);

/// The block at which the exported data was last updated.
pub const EXPORTED_DATA_LAST_BLOCK: IntGaugeVecDescriptor<4> = IntGaugeVecDescriptor::new(
    Some("task:alert"),
    "block_of_exported_value",
    "the_block_at_which_the_exported_data_was_last_updated",
    ["task_name", "key", "chain", "context"],
);

/// Number of events in buffer.
pub const TASK_BUFFERED_EVENTS: IntGaugeVecDescriptor<1> = IntGaugeVecDescriptor::new(
    Some("task"),
    "buffered_events",
    "number_of_events_in_buffer",
    ["task_name"],
);

/// MetricsRegistry struct.
#[derive(Debug, Clone)]
pub struct MetricsRegistry(Arc<Metrics>);

/// Implementation of MetricsRegistry.
impl MetricsRegistry {
    const METRICS_NAMESPACE: &'static str = "PHYLAX";

    /// Creates a new MetricsRegistry.
    pub fn new() -> Self {
        debug!(namespace = Self::METRICS_NAMESPACE, "New Metrics Registry");
        Self(Arc::new(Metrics::with_namespace(Self::METRICS_NAMESPACE)))
    }

    /// Returns the inner Arc<Metrics>.
    pub fn inner(&self) -> Arc<Metrics> {
        self.0.clone()
    }

    /// Sets the watcher network lag.
    pub fn set_watcher_network_lag(&self, task_name: &str, latency: i64) {
        let metric = self.0.igv(WATCHER_NETWORK_LAG).metric([task_name]);
        metric.set(latency);
    }

    /// Increments the task invocations.
    pub fn inc_task_invocations(&self, task_name: &str) {
        let metric = self.0.cv(TASK_INVOCATIONS).metric([task_name]);
        metric.inc();
    }

    /// Increments the activity errors.
    pub fn inc_activity_errors(&self, task_name: &str) {
        let metric = self.0.cv(TASK_ERRORS).metric([task_name]);
        metric.inc();
    }

    /// Sets the watcher block number.
    pub fn set_watcher_block_number(&self, task_name: &str, block: u64) {
        let metric = self.0.igv(WATCHER_BLOCK_NUMBER).metric([task_name]);
        metric.set(block as i64);
    }

    /// Sets the alert duration.
    pub fn set_alert_duration(&self, task_name: &str, duration: f64) {
        let metric = self.0.gv(ALERT_DURATION).metric([task_name]);
        metric.set(duration);
    }

    /// Sets the alert firing.
    pub fn set_alert_firing(&self, task_name: &str, firing: bool) {
        let metric = self.0.gv(ALERT_FIRING).metric([task_name]);
        if firing {
            metric.set(1.0);
        } else {
            metric.set(0.0);
        }
    }

    /// Sets the activity state for foreground.
    pub fn set_activity_state_fg(&self, task_name: &str, working: bool) {
        self.set_activity_state(task_name, working, true)
    }

    /// Sets the activity state for background.
    pub fn set_activity_state_bg(&self, task_name: &str, working: bool) {
        self.set_activity_state(task_name, working, false)
    }

    /// Sets the activity state.
    fn set_activity_state(&self, task_name: &str, working: bool, foreground: bool) {
        let work_type = if foreground { "foreground" } else { "background" };
        let metric = self.0.igv(TASK_WORKING).metric([task_name, work_type]);
        if working {
            metric.set(1);
        } else {
            metric.set(0);
        }
    }

    /// Gets the activity state.
    pub fn get_activity_state(&self, task_name: &str) -> (bool, bool) {
        let fg = self.0.igv(TASK_WORKING).metric([task_name, "foreground"]).get();
        let bg = self.0.igv(TASK_WORKING).metric([task_name, "background"]).get();
        (fg == 1, bg == 1)
    }

    /// Sets the exported data.
    pub fn set_exported_data(
        &self,
        task_name: &str,
        key: &str,
        chain: &str,
        context: &str,
        value: f64,
        block: u64,
    ) {
        let metric_value = self.0.gv(EXPORTED_DATA_VALUE).metric([task_name, key, chain, context]);
        let metric_last_block = self.0.igv(EXPORTED_DATA_LAST_BLOCK).metric([
            task_name,
            key,
            chain.to_string().as_str(),
            context,
        ]);
        metric_value.set(value);
        metric_last_block.set(block as i64);
    }

    /// Sets the task buffered events.
    pub fn set_task_buffered_events(&self, task_name: &str, value: usize) {
        let metric = self.0.igv(TASK_BUFFERED_EVENTS).metric([task_name]);
        metric.set(value as i64);
    }

    /// Gathers text.
    pub fn gather_text(&self) -> Result<Vec<u8>> {
        self.0.gather_text().map_err(|e| e.into())
    }

    /// Gets the watcher network lag.
    pub fn get_watcher_network_lag(&self, task_name: &str) -> i64 {
        let metric = self.0.igv(WATCHER_NETWORK_LAG).metric([task_name]);
        metric.get()
    }

    /// Gets the watcher block number.
    pub fn get_watcher_block_number(&self, task_name: &str) -> i64 {
        let metric = self.0.igv(WATCHER_BLOCK_NUMBER).metric([task_name]);
        metric.get()
    }

    /// Gets the alert firing.
    pub fn get_alert_firing(&self, task_name: &str) -> f64 {
        let metric = self.0.gv(ALERT_FIRING).metric([task_name]);
        metric.get()
    }

    /// Gets the alert duration.
    pub fn get_alert_duration(&self, task_name: &str) -> f64 {
        let metric = self.0.gv(ALERT_DURATION).metric([task_name]);
        metric.get()
    }

    /// Gets the task invocations.
    pub fn get_task_invocations(&self, task_name: &str) -> f64 {
        let metric = self.0.cv(TASK_INVOCATIONS).metric([task_name]);
        metric.get()
    }

    /// Gets the task errors.
    pub fn get_task_errors(&self, task_name: &str) -> f64 {
        let metric = self.0.cv(TASK_ERRORS).metric([task_name]);
        metric.get()
    }

    /// Gets the task working.
    pub fn get_task_working(&self, task_name: &str, work_type: &str) -> i64 {
        let metric = self.0.igv(TASK_WORKING).metric([task_name, work_type]);
        metric.get()
    }

    /// Gets the exported data value.
    pub fn get_exported_data_value(
        &self,
        task_name: &str,
        key: &str,
        chain: &str,
        context: &str,
    ) -> f64 {
        let metric = self.0.gv(EXPORTED_DATA_VALUE).metric([task_name, key, chain, context]);
        metric.get()
    }

    /// Gets the exported data last block.
    pub fn get_exported_data_last_block(
        &self,
        task_name: &str,
        key: &str,
        chain: &str,
        context: &str,
    ) -> i64 {
        let metric = self.0.igv(EXPORTED_DATA_LAST_BLOCK).metric([task_name, key, chain, context]);
        metric.get()
    }

    /// Gets the task buffered events.
    pub fn get_task_buffered_events(&self, task_name: &str) -> i64 {
        let metric = self.0.igv(TASK_BUFFERED_EVENTS).metric([task_name]);
        metric.get()
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
