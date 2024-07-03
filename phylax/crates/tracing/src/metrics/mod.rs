use opentelemetry::global;
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime,
};

use crate::error::PhylaxTracingError;

pub static NODE_METER_NAME: &str = "PhylaxNode";

/// Initalize the OpenTelemetry metrics provider and set it as the global provider.
///
/// This method starts up the OpenTelemetry metrics provider that the entire app will
/// use to track metrics.rate
pub fn init_metrics() -> Result<(), PhylaxTracingError> {
    let metrics_provider = init_provider()?;
    global::set_meter_provider(metrics_provider);
    Ok(())
}

/// This function initalizes the OpenTelemetry metrics providers and exporters.
fn init_provider() -> Result<SdkMeterProvider, PhylaxTracingError> {
    // TODO: Change this default SDK provider to one that uses the OTLP exporter once the Axum API
    // allows for exposing an OLTP endpoint.
    let exporter = opentelemetry_stdout::MetricsExporterBuilder::default().build();
    let reader = PeriodicReader::builder(exporter, runtime::Tokio).build();
    let provider = SdkMeterProvider::builder().with_reader(reader).build();
    Ok(provider)
}
