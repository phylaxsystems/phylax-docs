use once_cell::sync::Lazy;
use opentelemetry::{
    self, global,
    metrics::{Histogram, Unit},
    trace::FutureExt,
    Context as OtelContext, KeyValue,
};
use phylax_tracing::metrics::NODE_METER_NAME;

use crate::task::TaskDetails;

/// Per-task metrics provider that contains the specific metrics for a task.
pub struct TaskMetrics {
    /// The histogram that measures the duration of task processing that is triggered
    /// every time a message is received and proccessed, or when a watcher completes an execution.
    process_duration: Histogram<u64>,
    // TODO: Add per-task processing started, received message, sent message, sent none, and user
    // error counters, as well as event type counters.
    /// The OpenTelemetry context associated with the task, propagating details like the name and
    /// inner task type.
    pub otel_context: OtelContext,
}

/// Histogram that measures the duration of task processing that is triggered
/// every time a message is received and proccessed, or when a watcher completes an execution.
static PROCESS_MESSAGE_DURATION: Lazy<Histogram<u64>> = Lazy::new(|| {
    global::meter(NODE_METER_NAME)
        .u64_histogram("task.process.duration")
        .with_description("Measures duration of task processing in microseconds.")
        .with_unit(Unit::new("μs"))
        .init()
});

impl TaskMetrics {
    pub(crate) fn new(task_details: TaskDetails) -> Self {
        Self {
            process_duration: PROCESS_MESSAGE_DURATION.clone(),
            otel_context: OtelContext::new().with_value(task_details),
        }
    }

    pub(crate) fn record_processing(&self, duration: u64, attrs: &Vec<KeyValue>) {
        self.process_duration.record(duration, attrs.as_slice());
        ().with_context(self.otel_context.clone());
    }
}
