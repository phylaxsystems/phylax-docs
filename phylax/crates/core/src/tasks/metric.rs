use std::fmt;

use opentelemetry::{
    metrics::{
        Counter, Gauge, Histogram, InstrumentBuilder, Meter, MetricsError, Unit, UpDownCounter,
    },
    KeyValue,
};
use phylax_interfaces::activity::{MetricKind, MonitorConfig, PointValue};
use phylax_tracing::tracing::warn;

pub(crate) enum Metric<T> {
    Counter(Counter<T>),
    Gauge(Gauge<T>),
    Histogram(Histogram<T>),
    UpDownCounter(UpDownCounter<T>),
}

impl<T> Metric<T> {
    pub(crate) fn record(&self, value: T, attributes: &[KeyValue]) {
        match self {
            Metric::Counter(counter) => counter.add(value, attributes),
            Metric::Gauge(gauge) => gauge.record(value, attributes),
            Metric::Histogram(histogram) => histogram.record(value, attributes),
            Metric::UpDownCounter(up_down_counter) => up_down_counter.add(value, attributes),
        }
    }
}

pub(crate) enum NumericMetric {
    Float(Metric<f64>),
    Int(Metric<i64>),
    Uint(Metric<u64>),
}

impl fmt::Display for NumericMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumericMetric::Float(_) => {
                write!(f, "F64 Metric")
            }
            NumericMetric::Int(_) => {
                write!(f, "I64 Metric")
            }
            NumericMetric::Uint(_) => {
                write!(f, "U64 Metric")
            }
        }
    }
}

impl NumericMetric {
    pub(crate) fn from_config(
        monitor_config: &MonitorConfig,
        meter: &Meter,
    ) -> Option<NumericMetric> {
        match monitor_config {
            MonitorConfig { metric_type: None, .. } => None,
            MonitorConfig {
                value_type: PointValue::Float(_),
                metric_type: Some(metric_type),
                ..
            } => {
                let f64_metric = match metric_type {
                    MetricKind::Gauge => Metric::Gauge(init_metric(
                        meter.f64_gauge(monitor_config.name),
                        monitor_config,
                    )),
                    MetricKind::Counter => Metric::Counter(init_metric(
                        meter.f64_counter(monitor_config.name),
                        monitor_config,
                    )),
                    MetricKind::UpDownCounter => Metric::UpDownCounter(init_metric(
                        meter.f64_up_down_counter(monitor_config.name),
                        monitor_config,
                    )),
                    MetricKind::Histogram => Metric::Histogram(init_metric(
                        meter.f64_histogram(monitor_config.name),
                        monitor_config,
                    )),
                };
                Some(NumericMetric::Float(f64_metric))
            }
            MonitorConfig {
                value_type: PointValue::Int(_),
                metric_type: Some(metric_type),
                ..
            } => {
                let maybe_i64_metric = match metric_type {
                    MetricKind::Gauge => Some(Metric::Gauge(init_metric(
                        meter.i64_gauge(monitor_config.name),
                        monitor_config,
                    ))),
                    MetricKind::Counter => {
                        warn!(
                            monitor_name = monitor_config.name,
                            "Counter is not supported for signed values"
                        );
                        None
                    }
                    MetricKind::UpDownCounter => Some(Metric::UpDownCounter(init_metric(
                        meter.i64_up_down_counter(monitor_config.name),
                        monitor_config,
                    ))),
                    MetricKind::Histogram => {
                        warn!(
                            monitor_name = monitor_config.name,
                            "Histogram is not supported for signed values"
                        );
                        None
                    }
                };
                maybe_i64_metric.map(NumericMetric::Int)
            }
            MonitorConfig {
                value_type: PointValue::Uint(_),
                metric_type: Some(metric_type),
                ..
            } => {
                let maybe_u64_metric = match metric_type {
                    MetricKind::Gauge => Some(Metric::Gauge(init_metric(
                        meter.u64_gauge(monitor_config.name),
                        monitor_config,
                    ))),
                    MetricKind::Counter => Some(Metric::Counter(init_metric(
                        meter.u64_counter(monitor_config.name),
                        monitor_config,
                    ))),
                    MetricKind::UpDownCounter => {
                        warn!(
                            monitor_name = monitor_config.name,
                            "UpDownCounter is not supported for unsigned values"
                        );
                        None
                    }
                    MetricKind::Histogram => Some(Metric::Histogram(init_metric(
                        meter.u64_histogram(monitor_config.name),
                        monitor_config,
                    ))),
                };
                maybe_u64_metric.map(NumericMetric::Uint)
            }
        }
    }
}

fn init_metric<'a, B>(builder: InstrumentBuilder<'a, B>, monitor_config: &MonitorConfig) -> B
where
    B: TryFrom<InstrumentBuilder<'a, B>, Error = MetricsError>,
{
    builder
        .with_description(monitor_config.description)
        .with_unit(Unit::new(monitor_config.unit))
        .init()
}
