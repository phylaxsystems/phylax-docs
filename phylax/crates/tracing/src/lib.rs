use crate::{
    reth_tracing::{
        tracing::{
            metadata::LevelFilter,
            subscriber::{self, DefaultGuard},
            Subscriber,
        },
        FileWorkerGuard, LogFormat,
    },
    tracing_subscriber::{
        filter::Directive, layer::SubscriberExt, prelude::__tracing_subscriber_Layer,
        registry::LookupSpan, util::SubscriberInitExt, EnvFilter, Layer, Registry,
    },
};
use rolling_file::{RollingConditionBasic, RollingFileAppender};
use std::path::PathBuf;

// Re-export tracing crates
pub use reth_tracing;
pub use tracing;
pub use tracing_subscriber;

pub type BoxedLayer<S> = Box<dyn Layer<S> + Send + Sync>;

pub fn stdout<S>(
    default_directive: impl Into<Directive>,
    extra_targets: Vec<(&str, LevelFilter)>,
) -> BoxedLayer<S>
where
    S: Subscriber,
    for<'a> S: LookupSpan<'a>,
{
    let filter =
        EnvFilter::builder().with_default_directive(default_directive.into()).from_env().unwrap();

    let with_ansi = std::env::var("RUST_LOG_STYLE").map(|val| val != "never").unwrap_or(true);
    let with_target = std::env::var("RUST_LOG_TARGET").map(|val| val != "0").unwrap_or(true);

    let targets = tracing_subscriber::filter::Targets::new().with_targets(extra_targets);

    let target_layer =
        tracing_subscriber::fmt::layer().with_ansi(with_ansi).with_target(with_target).boxed();
    Box::new(targets.and_then(filter).and_then(target_layer))
}

/// Sets up temporary logging and returns a guard.
///
/// This function is used to set up temporary logging before the actual configuration is built.
/// The returned guard should be dropped after the actual logging is initialized.
pub fn setup_temp_logging() -> DefaultGuard {
    let temp_layer = stdout(LevelFilter::INFO, vec![("phylax", LevelFilter::INFO)]);
    let temp_subscriber = tracing_subscriber::registry().with(temp_layer);

    subscriber::set_default(temp_subscriber)
}

pub fn journald<S>(filter: EnvFilter) -> std::io::Result<BoxedLayer<S>>
where
    S: Subscriber,
    for<'a> S: LookupSpan<'a>,
{
    Ok(tracing_journald::layer()?.with_filter(filter).boxed())
}

// Taken from paradigmxyz/reth for compatibility, code licensed under MIT/Apache
pub fn file(
    filter: EnvFilter,
    log_format: LogFormat,
    logs_dir: &PathBuf,
    logs_file_name: &str,
    log_size_in_bytes: u64,
    log_max_files: usize,
) -> Result<(BoxedLayer<Registry>, FileWorkerGuard), Box<dyn std::error::Error>> {
    if !logs_dir.exists() {
        std::fs::create_dir_all(logs_dir)?;
    }
    let file_appender: RollingFileAppender<RollingConditionBasic> = RollingFileAppender::new(
        logs_dir.join(logs_file_name),
        RollingConditionBasic::new().max_size(log_size_in_bytes),
        log_max_files,
    )?;
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let layer = log_format.apply(filter, None, Some(file_writer));
    Ok((layer, guard))
}

/// Initializes a new [Subscriber] based on the given layers.
pub fn init(layers: Vec<BoxedLayer<Registry>>) {
    // This may emit an error if the global tracing subscriber has already been setup,
    // which is an expected state and is safe to not handle.
    let _ = tracing_subscriber::registry().with(layers).try_init();
}
