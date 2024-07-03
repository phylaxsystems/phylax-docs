use crate::{
    context::ActivityContext,
    error::{PhylaxError, PhylaxTaskError},
    message::{BoxedMessage, MessageDetails},
};
use std::{
    any::{Any, TypeId},
    future::Future,
    sync::{atomic::AtomicU64, Arc},
};
use tokio_util::sync::ReusableBoxFuture;

/// Trait defining the underlying behavior and lifecycle for actors that run as tasks in the Phylax
/// Node.
///
/// The Activity trait is a core part of Phylax. It represents the interface for which all work
/// defined to run as part of the actor framework will do during execution. Activities in this
/// actor framework live as long as the entire runtime, have work triggered by input messages which
/// contents are defined as the associated `Input` type of the trait. Activities will receive input
/// messages from all other activities that have the same input type marked to be their associated
/// `Output` type.
///
/// All activities are expected to have at least one accompanying activity with an input type for
/// their output type, or output type for their input type, otherwise the Phylax Node will fail
/// to launch. However, there are categories of activities that are not expected to have either an
/// input or output, called "Watchers" and "Actions" (all other activities are considered "Alerts").
/// The unit type, or `()`, is treated specially by Phylax and will cause the behavior of the
/// activity to change when set as either the `Input` or `Output` type for a Watcher and Action
/// respectively.
///
/// Normally for Alerts, tasks will await for input messages of the specified `Input` type to
/// become available for processing, and then output a message as the `Output` type for other
/// activities to process. For Watchers, this means that the activity `process_message` function
/// will no triggered by input messages and will instead be run indefinitely in a loop. For
/// Actions, their `process_message` work will still be triggered by an input event, but there it
/// has no way to communicate the output of their behavior to any other activity directly through
/// the normal message passing channels.
///
/// Lifecycles of tasks and activities can also be customized through the trait by implementing
/// the `bootstrap` and `cleanup` methods. These methods are only called during task startup or
/// shutdown, and represent a guaranteed way to ensure certain behaviors happen even after a panic.
///
/// # Example
///
/// ```
/// # use tokio::time::Duration;
/// # use phylax_interfaces::{activity::Activity, context::ActivityContext, message::MessageDetails, error::PhylaxError};
/// pub struct Message;
/// pub struct Watcher;
/// impl Activity for Watcher {
///     const FLAVOR_NAME: &'static str = "example_watcher_flavor";
///     type Input = ();
///     type Output = Message;
///
///     async fn bootstrap(&mut self, context: ActivityContext) -> Result<(), PhylaxError> {
///         // Some setup logic here
///         Ok(())
///     }
///
///     async fn process_message(
///         &mut self,
///         _input: Option<(Self::Input, MessageDetails)>,
///         context: ActivityContext,
///     ) -> Result<Option<Self::Output>, PhylaxError> {
///         // Some .await point or timeout
///         tokio::time::sleep(Duration::from_millis(500)).await;
///         Ok(Some(Message))
///     }
///
///     async fn cleanup(&mut self, context: ActivityContext) -> Result<(), PhylaxError> {
///         // Some shutdown logic here
///         Ok(())
///     }
/// }
///
/// pub struct Action;
/// impl Activity for Action {
///     const FLAVOR_NAME: &'static str = "example_action_flavor";
///     type Input = Message;
///     type Output = ();
///
///     async fn process_message(
///         &mut self,
///         input: Option<(Self::Input, MessageDetails)>,
///         context: ActivityContext,
///     ) -> Result<Option<Self::Output>, PhylaxError> {
///         // Some logic based on the input, which is received from a `Watcher` activity.
///         Ok(None)
///     }
/// }
/// ```
pub trait Activity: Send {
    /// The name for the kind of activity, which a single activity implementation may have
    /// multiple concrete instances refer to. The flavor name is used in displays, logging, and
    /// metrics.
    const FLAVOR_NAME: &'static str;
    /// A static slice of all labels for the activity, which are used to decorate the
    /// GUI representation of the Activity.
    const LABELS: &'static [Label] = &[];
    /// A static slice of all monitor configurations for the activity, which are used for
    /// visualization, metrics, data exports, and custom health alerting.
    const MONITORS: &'static [MonitorConfig] = &[];
    /// The type of the messages that will be received by the activity from other activities.
    type Input: Any + Send;
    /// The type of the messages that will be sent by the activity to other activities.
    type Output: Any + Send;

    /// Overwritable method that can be used to do work once during the startup phase of the Phylax
    /// node.
    fn bootstrap(
        &mut self,
        _context: ActivityContext,
    ) -> impl Future<Output = Result<(), PhylaxError>> + Send {
        async { Ok(()) }
    }

    /// Method for defining all repeating work done by an Activity during the Phylax node
    /// execution.
    ///
    /// This function is run inside of a loop for activities, but behaves differently for different
    /// activity types. Watchers will always see the `input` parameter contain `None`, and will run
    /// this function repeatedly until there is an await inside of it. This can easily starve your
    /// Phylax node task runtime if there are many watchers without any await points that last for
    /// a reasonable amount of time.
    ///
    /// For Alerts and Actions, this function will run only after a message has arrived in the
    /// receiver channels, and will spawn a separate task to execute the relevant work for it
    /// as well. The `input` parameter for these types will always contain a message.
    ///
    /// It's possible to return `None` or a `Some(JoinHandle())`, which itself can contain a
    /// `Ok(Output)`, `Err(UnrecoverableError)`, or `Err(RecoverableError)` variants. The `None`
    /// return is effectively how the runtime will always treat the return type of Action
    /// activities, but can be returned by the other types to represent a no-op as well. The
    /// `Ok` result is normally how Watchers and Alerts need to return output messages, but if
    /// they also need to indicate a failure they can return the [`PhylaxError`] variants. The
    /// `UnrecoverableError` will cause the Phylax node to attempt to gracefully shutdown, but
    /// the `RecoverableError` variant will simply log the error and allow the task to continue
    /// processing.
    fn process_message(
        &mut self,
        input: Option<(Self::Input, MessageDetails)>,
        context: ActivityContext,
    ) -> impl Future<Output = Result<Option<Self::Output>, PhylaxError>> + Send;

    /// Overwritable method that can be used to do work once during the shutdown phase of the Phylax
    /// node.
    fn cleanup(
        &mut self,
        _context: ActivityContext,
    ) -> impl Future<Output = Result<(), PhylaxError>> + Send {
        async { Ok(()) }
    }
}

/// A struct representing a key-value pair label.
///
/// This struct is used to store metadata about an activity in the form of key-value pairs,
/// which is primarily used for enriching the UI for viewing an Activity.
///
/// # Fields
///
/// * `key` - A static string slice that holds the key of the label.
/// * `value` - A static string slice that holds the value of the label.
#[derive(Debug)]
pub struct Label {
    pub key: &'static str,
    pub value: &'static str,
}

/// Configuration for an [`Activity`] monitor, which allows for data unrelated to the `Output`
/// message of a Node to be persisted, exported to OpenTelemetry, or visualized in the Phylax node's
/// GUI.
///
/// /// # Fields
///
/// * `name` - The unique namespace of the monitor.
/// * `display` - The [`Visualization`] type for the monitored data.
/// * `category` - The [`MonitorKind`] of the monitor - whether it represented health, or a normal
///   monitor.
/// * `metric_type` - The optional [`MetricKind`] of the monitor - whether it creates an exported
///   OpenTelemetry gauge, counter, updowncounter, or histogram.
/// * `value_type` - The [`PointValue`] of the monitor - whether it was a float, int, or bool.
///
/// # Example
///
/// ```
/// use phylax_interfaces::{activity::{Activity, PointValue, MonitorConfig, Visualization, MetricKind, MonitorKind, TimeSeriesConfig}, context::{ActivityContext, HealthSeverity}, message::MessageDetails, error::PhylaxError};
/// # use std::time::{Instant, Duration};
/// # use std::collections::VecDeque;
/// # pub struct Message;
/// pub struct Watcher {
///     last_5_latencies: VecDeque<u64>,
/// }
/// const WATCHER_LATENCY_MONITOR_NAME: &'static str = "ImportantHealthMovingAvg";
///
/// const MONITOR_CONFIGS : &'static [MonitorConfig] = &[
///     MonitorConfig {
///         name: "RandomExportData",
///         description: "A random exported data point",
///         unit: "ms",
///         metric_type: Some(MetricKind::Gauge),
///         value_type: PointValue::Float(0.0),
///         display: None,
///         category: MonitorKind::Default,
///     },
///     MonitorConfig {
///         name: WATCHER_LATENCY_MONITOR_NAME,
///         description: "The moving average of the watcher's latency over the last 5 seconds",
///         unit: "ms",
///         metric_type: Some(MetricKind::Gauge),
///         value_type: PointValue::Float(0.0),
///         display: Some(Visualization::TimeSeries(TimeSeriesConfig {
///             x_axis_name: "Time",
///             y_axis_name: "Watcher Latency (ms)",
///         })),
///         category: MonitorKind::Health { unhealthy_message: "The watcher has not emitted a message in the last 5 seconds" },
///     },
/// ];
///
/// impl Activity for Watcher {
///     const FLAVOR_NAME: &'static str = "WatcherActivity";
///     const MONITORS: &'static [MonitorConfig] = MONITOR_CONFIGS;
///     type Input = ();
///     type Output = Message;
///
///     async fn process_message(
///         &mut self,
///         _input: Option<(Self::Input, MessageDetails)>,
///         context: ActivityContext,
///     ) -> Result<Option<Self::Output>, PhylaxError> {
///         let logic_start_time = Instant::now();
///         // Some watcher logic here...
///         let run_latency = Instant::now().duration_since(logic_start_time).as_millis() as u64;
///         
///         // Build moving average from last 5 latencies
///         self.last_5_latencies.push_back(run_latency);
///         if self.last_5_latencies.len() > 5 {
///             self.last_5_latencies.pop_front();
///         }
///         let moving_average = self.last_5_latencies.iter().sum::<u64>() as f64
///             / self.last_5_latencies.len() as f64;
///
///         let severity = if moving_average > 5000.0 {
///             HealthSeverity::Critical
///         } else {
///             HealthSeverity::Warning
///         };
///         
///         context.health_monitors.get(WATCHER_LATENCY_MONITOR_NAME).unwrap()
///             .send_f64(moving_average, severity);
///         context.activity_monitors.get(WATCHER_LATENCY_MONITOR_NAME).unwrap()
///             .send(PointValue::Float(0.00729));
///         Ok(None)
///     }
/// }
/// ```
pub struct MonitorConfig {
    /// The name of the monitor to be namespaced in OpenTelemetry metrics and to be the title of
    /// any visualizations in the GUI.
    pub name: &'static str,
    /// A short description of the monitor to be displayed in the GUI and in metrics.
    pub description: &'static str,
    /// The unit of the monitored data to be displayed in the GUI and in metrics.
    pub unit: &'static str,
    /// The visualization type for the monitored data as it might be graphed in the GUI.
    pub display: Option<Visualization>,
    /// The type of metric for the monitored data to be exported through OpenTelemetry.
    pub metric_type: Option<MetricKind>,
    /// The data type of the monitor's datapoints, as well as an initial value if applicable.
    pub value_type: PointValue,
    /// The category of the monitor, which determines how the monitor is displayed in the GUI and
    /// interacts with Phylax notifications.
    pub category: MonitorKind,
}

/// Enum representing the type of visualization to be used for monitoring data.
///
/// The `Visualization` enum provides different options for how data can be visualized
/// in the Phylax node's GUI. Each variant represents a different type of visualization
/// that can be configured for a monitor.
pub enum Visualization {
    /// A time series graph that plots data points over time.
    TimeSeries(TimeSeriesConfig),
    // TODO: Add HeatMap specific visualization configuration.
    HeatMap,
}

/// The category of a monitor, which determines how the monitor will be displayed in the Phylax
/// node's GUI or interact with Phylax notifications.
pub enum MonitorKind {
    /// A monitor that allows data to be charted and exported via metrics and logs, but doesn't
    /// have any other behavior.
    Default,
    /// A monitor that specifically can track the health of an [`Activity`] in addition to
    /// the functionality of the default monitor type, of the message that can be displayed if the
    /// monitor enters an unhealth or critical state.
    Health { unhealthy_message: &'static str },
}

/// Enum representing the type of metric to be used for monitoring data.
///
/// The `MetricKind` enum provides different options for how data can be exported
/// through OpenTelemetry. Each variant represents a different type of metric
/// that can be configured for a monitor.
pub enum MetricKind {
    /// A gauge metric, which represents a single numerical value at a given point in time.
    Gauge,
    /// A counter metric, which represents a cumulative value that only increases.
    Counter,
    /// An up-down counter metric, which represents a cumulative value that can increase and
    /// decrease.
    UpDownCounter,
    // TODO: Allow histograms to set their own bucket values via MonitorConfig.
    /// A histogram metric, which represents the distribution of a set of values.
    Histogram,
}

/// A the type of datapoint to be added to a monitor's dataset. This is used for both
/// [`MonitorCategory::Default`] and [`MonitorCategory::Health`] data, and allows for a default
/// initial value to be set if applicable.
#[derive(Clone, Debug)]
pub enum PointValue {
    /// A floating-point value type for the monitor dataset.
    Float(f64),
    /// A signed integer value type for the monitor dataset.
    Int(i64),
    /// An unsigned integer value type for the monitor dataset.
    Uint(u64),
}

/// Configuration for a time series visualization.
pub struct TimeSeriesConfig {
    /// The name of the x-axis which represents time.
    pub x_axis_name: &'static str,
    /// The name of the y-axis which represents data point values.
    pub y_axis_name: &'static str,
}

/// Wrapper trait which erases associated types and allows for object safe Activities.
///
/// `DynActivity` is the interface with which the internal task system uses to interact
/// with [`Activity`] traits, and allows for them to be used as trait objects without
/// leaking information about their associated types.
///
/// Additionally, the `DynActivity` trait provides some further basic functionality
/// that all actions for interacting with an activity will be required, such as
/// deconstructing and decorating the [`BoxedMessage`] struct before and after the
/// message is consumed by the [`Activity`].
///
/// The return type of the async functions of this trait are [`ReusableBoxFuture`], which allow
/// the futures to contain dynamically sized objects, like would be possible through the
/// `Pin<Box<dyn Future<Output = T> + Send + 'a>>` return type.
///
/// This trait should not be used to extend Phylax Activities and Tasks, instead
/// use the `Activity` trait in the `interfaces` crate.
pub trait DynActivity: Send {
    fn bootstrap(
        &mut self,
        context: ActivityContext,
    ) -> ReusableBoxFuture<Result<(), PhylaxTaskError>>;
    fn process_message(
        &mut self,
        input: Option<BoxedMessage>,
        context: ActivityContext,
        message_counter: Arc<AtomicU64>,
    ) -> ReusableBoxFuture<Result<Option<BoxedMessage>, PhylaxTaskError>>;
    fn cleanup(
        &mut self,
        context: ActivityContext,
    ) -> ReusableBoxFuture<Result<(), PhylaxTaskError>>;
    fn get_input_type_id(&self) -> TypeId;
    fn get_output_type_id(&self) -> TypeId;
    fn get_activity_flavor(&self) -> &'static str;
    fn get_monitor_configs(&self) -> &'static [MonitorConfig];
    fn get_labels(&self) -> &'static [Label];
    fn get_activity_name(&self) -> String;
}
