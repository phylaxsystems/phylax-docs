use color_eyre::Result;
use phylax_tracing::{
    reth_tracing::{
        tracing::metadata::LevelFilter,
        tracing_subscriber::{filter::Directive, EnvFilter},
        FileWorkerGuard, LogFormat,
    },
    tracing_subscriber::Registry,
    BoxedLayer,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
    str::FromStr,
};

use crate::substitute_env::deserialize_with_env;

/// Struct for Tracing Configuration
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct TracingConfig {
    /// The log level of the application. One of:
    /// off | info | warn | error | debug | trace
    #[serde(deserialize_with = "deserialize_with_env")]
    pub log_level: LogLevel,
    /// Path to the log file
    #[serde(deserialize_with = "deserialize_with_env")]
    pub logs_dir: PathBuf,
    /// Name of the log file
    #[serde(deserialize_with = "deserialize_with_env")]
    pub logs_file: String,
    /// Flag to indicate if logs are persistent. Persistent logs are stored in the log file
    pub persistent: bool,
    /// Flag to indicate if logs are sent to journald
    pub journald: bool,
    /// Filter string for logs. This is the same filter that is used in `RUST_LOG`
    #[serde(deserialize_with = "deserialize_with_env")]
    pub filter: String,
    /// The maximum size (in MB) of one log file.
    pub log_file_max_size: u64,
    /// The maximum amount of log files that will be stored. If set to 0, background file logging
    /// is disabled.
    pub log_file_max_files: usize,
}

/// Enum for Logging level
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    /// Off
    Off,
    /// Error
    Error,
    /// Warn
    Warn,
    /// Debug
    Debug,
    /// Trace
    Trace,
    /// Info
    #[serde(other)]
    #[default]
    Info,
}

/// Default implementation for TracingConfig
impl Default for TracingConfig {
    fn default() -> Self {
        let default_level: LogLevel = Default::default();
        Self {
            logs_dir: crate::dirs::logs_dir(),
            logs_file: Self::DEFAULT_LOGS_FILE.to_owned(),
            log_level: default_level,
            journald: false,
            persistent: true,
            filter: default_level.to_string(),
            log_file_max_size: 200,
            log_file_max_files: 5,
        }
    }
}

/// Constant to convert megabytes to bytes
const MB_TO_BYTES: u64 = 1024 * 1024;

/// Implementation of TracingConfig
impl TracingConfig {
    pub const DEFAULT_LOGS_FILE: &'static str = "phylax.log";

    /// Builds a tracing layer from the current log options.
    /// Taken from Reth, licensed under MIT
    pub fn layer(&self) -> eyre::Result<Option<(BoxedLayer<Registry>, Option<FileWorkerGuard>)>> {
        let filter = EnvFilter::builder().parse(&self.filter)?;
        if self.journald {
            let layer = phylax_tracing::journald(filter).expect("Could not connect to journald");
            Ok(Some((layer, None)))
        } else if self.persistent {
            let (layer, guard) = phylax_tracing::file(
                filter,
                LogFormat::Terminal,
                &self.logs_dir,
                self.logs_file.as_str(),
                self.log_file_max_size * MB_TO_BYTES,
                self.log_file_max_files,
            )
            .expect("Could not initialize file logging");
            Ok(Some((layer, Some(guard))))
        } else {
            Ok(None)
        }
    }
}

/// Implementation of FromStr for LogLevel
impl FromStr for LogLevel {
    type Err = eyre::Report;
    /// Converts a string to LogLevel
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "off" => Ok(LogLevel::Off),
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(eyre::Report::msg("Incorrect LogLevel")),
        }
    }
}

/// Implementation of Display for LogLevel
impl Display for LogLevel {
    /// Formats LogLevel for display
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Off => write!(f, "off"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Info => write!(f, "info"),
        }
    }
}

/// Implementation of From for LogLevel to Directive
impl From<LogLevel> for Directive {
    /// Converts LogLevel to Directive
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Directive::from(LevelFilter::TRACE),
            LogLevel::Off => Directive::from(LevelFilter::OFF),
            LogLevel::Warn => Directive::from(LevelFilter::WARN),
            LogLevel::Error => Directive::from(LevelFilter::ERROR),
            LogLevel::Debug => Directive::from(LevelFilter::DEBUG),
            LogLevel::Info => Directive::from(LevelFilter::INFO),
        }
    }
}

/// Implementation of From for LogLevel to LevelFilter
impl From<LogLevel> for LevelFilter {
    /// Converts LogLevel to LevelFilter
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Off => LevelFilter::OFF,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
        }
    }
}
