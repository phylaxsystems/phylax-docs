// Code reused from paradigm/reth reth-tracing crate and licensed under MIT.
// This code is re-licensed and re-distributed by Phylax in the root of this repo.
use std::{
    io,
    path::{Path, PathBuf},
};

use rolling_file::{RollingConditionBasic, RollingFileAppender};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter::Directive, EnvFilter, Layer, Registry};

use crate::{
    error::PhylaxTracingError,
    trace::{BoxedLayer, LogFormat},
};

/// A worker guard returned by the file layer.
///
///  When a guard is dropped, all events currently in-memory are flushed to the log file this guard
///  belongs to.
pub type FileWorkerGuard = tracing_appender::non_blocking::WorkerGuard;

const PHYLAX_LOG_FILE_NAME: &str = "phylax.log";

/// Manages the collection of layers for a tracing subscriber.
///
/// `Layers` acts as a container for different logging layers such as stdout, file, or journald.
/// Each layer can be configured separately and then combined into a tracing subscriber.
#[derive(Default)]
pub struct Layers {
    inner: Vec<BoxedLayer<Registry>>,
}

impl Layers {
    /// Creates a new `Layers` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes the `Layers` instance, returning the inner vector of layers.
    pub fn into_inner(self) -> Vec<BoxedLayer<Registry>> {
        self.inner
    }

    /// Adds a journald layer to the layers collection.
    ///
    /// # Arguments
    /// * `filter` - A string containing additional filter directives for this layer.
    ///
    /// # Returns
    /// An `Result<(), PhylaxTracingError>` indicating the success or failure of the operation.
    pub fn journald(&mut self, filter: &str) -> Result<(), PhylaxTracingError> {
        let journald_filter = build_env_filter(None, filter)?;
        let layer = tracing_journald::layer()
            .map_err(|e| PhylaxTracingError::InitError(e.into()))?
            .with_filter(journald_filter)
            .boxed();
        self.inner.push(layer);
        Ok(())
    }

    /// Adds a stdout layer with specified formatting and filtering.
    ///
    /// # Type Parameters
    /// * `S` - The type of subscriber that will use these layers.
    ///
    /// # Arguments
    /// * `format` - The log message format.
    /// * `directive` - Directive for the default logging level.
    /// * `filter` - Additional filter directives as a string.
    /// * `color` - Optional color configuration for the log messages.
    ///
    /// # Returns
    /// An `Result<(), PhylaxTracingError>` indicating the success or failure of the operation.
    pub fn stdout(
        &mut self,
        format: LogFormat,
        default_directive: Directive,
        filters: &str,
        color: Option<String>,
    ) -> Result<(), PhylaxTracingError> {
        let filter = build_env_filter(Some(default_directive), filters)?;
        let layer = format.apply(filter, color, None);
        self.inner.push(layer.boxed());
        Ok(())
    }

    /// Adds a file logging layer to the layers collection.
    ///
    /// # Arguments
    /// * `format` - The format for log messages.
    /// * `filter` - Additional filter directives as a string.
    /// * `file_info` - Information about the log file including path and rotation strategy.
    ///
    /// # Returns
    /// An `Result<FileWorkerGuard, PhylaxTracingError>` representing the file logging worker.
    pub fn file(
        &mut self,
        format: LogFormat,
        filter: &str,
        file_info: FileInfo,
    ) -> Result<FileWorkerGuard, PhylaxTracingError> {
        let (writer, guard) =
            file_info.create_log_writer().map_err(|e| PhylaxTracingError::InitError(e.into()))?;
        let file_filter = build_env_filter(None, filter)?;
        let layer = format.apply(file_filter, None, Some(writer));
        self.inner.push(layer);
        Ok(guard)
    }
}

/// Holds configuration information for file logging.
///
/// Contains details about the log file's path, name, size, and rotation strategy.
#[derive(Debug, Clone)]
pub struct FileInfo {
    dir: PathBuf,
    file_name: String,
    max_size_bytes: u64,
    max_files: usize,
}

impl FileInfo {
    /// Creates a new `FileInfo` instance.
    pub fn new(dir: PathBuf, max_size_bytes: u64, max_files: usize) -> Self {
        Self { dir, file_name: PHYLAX_LOG_FILE_NAME.to_string(), max_size_bytes, max_files }
    }

    /// Creates the log directory if it doesn't exist.
    ///
    /// # Returns
    /// A reference to the path of the log directory.
    fn create_log_dir(&self) -> io::Result<&Path> {
        let log_dir: &Path = self.dir.as_ref();
        if !log_dir.exists() {
            std::fs::create_dir_all(log_dir)?;
        }
        Ok(log_dir)
    }

    /// Creates a non-blocking writer for the log file.
    ///
    /// # Returns
    /// A tuple containing the non-blocking writer and its associated worker guard.
    fn create_log_writer(
        &self,
    ) -> io::Result<(tracing_appender::non_blocking::NonBlocking, WorkerGuard)> {
        let log_dir = self.create_log_dir()?;
        let (writer, guard) = tracing_appender::non_blocking(RollingFileAppender::new(
            log_dir.join(&self.file_name),
            RollingConditionBasic::new().max_size(self.max_size_bytes),
            self.max_files,
        )?);
        Ok((writer, guard))
    }
}

/// Builds an environment filter for logging.
///
/// The events are filtered by `default_directive`, unless overridden by `RUST_LOG`.
///
/// # Arguments
/// * `default_directive` - An optional `Directive` that sets the default directive.
/// * `directives` - Additional directives as a comma-separated string.
///
/// # Returns
/// An `Result<EnvFilter, PhylaxTracingError>` that can be used to configure a tracing subscriber.
fn build_env_filter(
    default_directive: Option<Directive>,
    directives: &str,
) -> Result<EnvFilter, PhylaxTracingError> {
    let env_filter = if let Some(default_directive) = default_directive {
        EnvFilter::builder().with_default_directive(default_directive).from_env_lossy()
    } else {
        EnvFilter::builder().from_env_lossy()
    };

    directives.split(',').filter(|d| !d.is_empty()).try_fold(env_filter, |env_filter, directive| {
        Ok(env_filter.add_directive(directive.parse()?))
    })
}
