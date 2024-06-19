//! CLI definition and entrypoint to executable
use std::path::PathBuf;

use crate::{
    debug, dirs,
    node::{self},
    runner::CliRunner,
};
use clap::{ArgAction, Args, Parser, Subcommand};
use phylax_config::{LogLevel, PhConfig, PhConfigBuilder};
use phylax_tracing::{
    reth_tracing::FileWorkerGuard,
    tracing::{info, metadata::LevelFilter},
};
use serde::Serialize;
/// This is the entrypoint to the executable.
#[derive(Debug, Parser, Clone, Serialize)]
#[command(author, version = crate::version::SHORT_VERSION, long_version = crate::version::LONG_VERSION, about = "Phylax", long_about = None)]
pub struct Cli {
    /// The command to run
    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    logs: Logs,

    #[clap(flatten)]
    verbosity: Verbosity,

    #[clap(flatten)]
    pub config: Config,
}

impl Cli {
    /// Execute the configured cli command.
    pub fn run(self) -> eyre::Result<()> {
        color_eyre::install()?;
        let _guard = phylax_tracing::setup_temp_logging();
        // setup default temp logging to build the config
        self.load_dotenv();
        let runner = CliRunner::default();
        let config = self.build_config()?;
        // Now setup logging based on the config
        drop(_guard);
        let _file_guard = self.init_tracing(&config)?;
        match self.command.clone() {
            // Run the command until ctrl-c is pressed and then spawn both on a tokio runtime
            Commands::Node(command) => runner.run_command_until_exit(command.execute(config))?,
            Commands::Debug(command) => runner.run_command_until_exit(command.execute(config))?,
        };
        phylax_tracing::tracing::info!("Hasta la vista, baby");
        Ok(())
    }

    /// Initializes tracing with the configured options.
    ///
    /// If file logging is enabled, this function returns a guard that must be kept alive to ensure
    /// that all logs are flushed to disk.
    pub fn init_tracing(&self, config: &PhConfig) -> eyre::Result<Option<FileWorkerGuard>> {
        let level: LevelFilter = config.tracing.log_level.into();
        let mut extra_filters = match config.tracing.log_level {
            LogLevel::Trace | LogLevel::Debug => {
                vec![
                    ("forge", level),
                    ("ethers_providers", level),
                    ("hyper", level),
                    ("script", level),
                    ("test", level),
                ]
            }
            _ => vec![("forge", LevelFilter::OFF), ("etherscanidentifier", LevelFilter::OFF)],
        };
        // We always want **all** the logs from phylax & sisyphus
        extra_filters.push(("phylax", LevelFilter::TRACE));
        extra_filters.push(("sisyphus", LevelFilter::TRACE));
        extra_filters.push(("tower_http", LevelFilter::TRACE));
        let stdout = phylax_tracing::stdout(config.tracing.log_level, extra_filters.clone());
        let mut layers = vec![stdout];
        let file_guard = config.tracing.layer()?.map(|(layer, guard)| {
            layers.push(layer);
            guard
        });
        phylax_tracing::init(layers);
        Ok(file_guard.flatten())
    }

    /// Build the config with the following order:
    /// - default
    /// - config file
    /// - cli args
    pub fn build_config(&self) -> eyre::Result<PhConfig> {
        let mut builder = PhConfigBuilder::default().with_source_from_default();
        builder = builder.with_source_from_path(self.config.local_config.clone());
        self.apply_cli(builder).and_then(|b| b.try_build())
    }

    pub fn load_dotenv(&self) {
        let current_path = std::env::current_dir().unwrap();
        let str = current_path.to_string_lossy();
        let res = dirs::load_dotenv_from_dir(current_path.as_path());
        if res.is_ok() {
            info!(env_path = %str, "Loaded .env from current working directory");
        } else {
            let stringified = self.config.local_config.to_string_lossy();
            let dir = self.config.local_config.parent().unwrap();
            let res = dirs::load_dotenv_from_dir(dir);
            if res.is_ok() {
                info!(env_path=%stringified, "Loaded .env from config directory");
            }
        }
    }

    fn apply_cli(&self, mut builder: PhConfigBuilder) -> eyre::Result<PhConfigBuilder> {
        // CLI globals
        builder =
            builder.set_override("tracing.logLevel", self.verbosity.log_level().to_string())?;
        builder = builder.set_override("tracing.journald", self.logs.journald)?;
        builder = builder.set_override_option("tracing.filter", self.logs.filter.clone())?;
        builder = builder.set_override_option(
            "tracing.logsDir",
            self.logs.log_directory.clone().map(|p| p.display().to_string()),
        )?;
        builder = builder.set_override("tracing.persistent", self.logs.persistent)?;
        builder =
            builder.set_override_option("tracing.logFileMaxSize", self.logs.log_file_max_size)?;
        builder =
            builder.set_override_option("tracing.logFileMaxFiles", self.logs.log_file_max_files)?;
        builder = match self.command.clone() {
            Commands::Node(node) => {
                builder = builder.set_override("api.enableApi", node.enable_api)?;
                builder = builder.set_override("api.enableMetrics", node.enable_metrics)?;
                builder = builder.set_override_option("api.jwtSecret", node.jwt_secret)?;
                builder.set_override_option("api.bindIp", node.bind_ip.map(|p| p.to_string()))?
            }
            _ => builder,
        };
        builder = builder.set_override(
            "source",
            self.config.local_config.clone().to_string_lossy().to_string(),
        )?;
        Ok(builder)
    }
}

/// Convenience function for parsing CLI options, set up logging and run the chosen command.
#[inline]
pub fn run() -> eyre::Result<()> {
    Cli::parse().run()
}

/// Commands to be executed
#[derive(Debug, Subcommand, Clone, Serialize)]
pub enum Commands {
    /// Start the node
    #[command(name = "node")]
    Node(node::NodeCommand),
    // /// Print the config to stdout
    // #[command(name = "config")]
    // Config(config::Command),
    // /// Test the tasks by bootstraping them
    #[command(name = "debug")]
    Debug(debug::DebugCommand),
}

#[derive(Debug, Clone, Args, Serialize, Default)]
#[command(next_help_heading = "Config")]
pub struct Config {
    #[arg(long = "config.file", value_name = "PATH", verbatim_doc_comment)]
    /// The path to the local config file
    pub local_config: PathBuf,
}

impl Config {
    pub fn default() -> Self {
        Config { local_config: PathBuf::from("./phylax.yaml") }
    }

    pub fn new(local_config: PathBuf) -> Self {
        Config { local_config }
    }
}

/// The log configuration.
#[derive(Debug, Clone, Args, Serialize, Default)]
#[command(next_help_heading = "Logging")]
pub struct Logs {
    /// The flag to enable persistent logs.
    #[arg(long = "log.persistent", global = true, conflicts_with = "journald")]
    pub persistent: bool,

    /// The path to put log files in.
    #[arg(long = "log.directory", value_name = "PATH", global = true, conflicts_with = "journald")]
    pub log_directory: Option<PathBuf>,

    /// Log events to journald.
    #[arg(long = "log.journald", global = true, conflicts_with = "log_directory")]
    pub journald: bool,

    /// The filter to use for logs written to the log file.
    #[arg(long = "log.filter", value_name = "FILTER", global = true)]
    pub filter: Option<String>,

    /// The maximum size (in MB) of one log file.
    #[arg(long = "log.file_max_size", value_name = "MAX_SIZE", global = true)]
    pub log_file_max_size: Option<u64>,

    /// The maximum amount of log files that will be stored. If set to 0, background file logging
    /// is disabled.
    #[arg(long = "log.file_max_files", value_name = "MAX_FILES", global = true)]
    pub log_file_max_files: Option<u64>,
}

/// The verbosity settings for the cli.
#[derive(Debug, Copy, Clone, Args, Serialize)]
#[command(next_help_heading = "Display")]
pub struct Verbosity {
    /// Set the minimum log level.
    ///
    /// -v      Errors
    /// -vv     Warnings
    /// -vvv    Info
    /// -vvvv   Debug
    /// -vvvvv  Traces (warning: very verbose!)
    #[clap(short, long, action = ArgAction::Count, global = true, default_value_t = 3, verbatim_doc_comment, help_heading = "Display")]
    verbosity: u8,

    /// Silence all log output.
    #[clap(long, alias = "silent", short = 'q', global = true, help_heading = "Display")]
    quiet: bool,
}

impl Verbosity {
    /// Get the corresponding [LogLevel] for the given verbosity, or none if the verbosity
    /// corresponds to silent.
    pub fn log_level(&self) -> LogLevel {
        if self.quiet {
            LogLevel::Off
        } else {
            match self.verbosity - 1 {
                0 => LogLevel::Error,
                1 => LogLevel::Warn,
                2 => LogLevel::Info,
                3 => LogLevel::Debug,
                _ => LogLevel::Trace,
            }
        }
    }
}

impl Default for Verbosity {
    fn default() -> Self {
        Verbosity { verbosity: 3, quiet: false }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::node::NodeCommand;

    use super::*;
    use phylax_config::PhConfigBuilder;
    use tempfile::tempdir;

    #[test]
    fn test_apply_cli() {
        let builder = PhConfigBuilder::default().with_source_from_default();
        let bind = ([127, 1, 1, 3], 9123);
        let node = NodeCommand {
            bind_ip: Some(bind.into()),
            enable_api: true,
            enable_metrics: true,
            jwt_secret: Some("test secret".to_owned()),
        };
        let cli = Cli {
            command: Commands::Node(node),
            logs: Logs {
                persistent: true,
                log_directory: Some(PathBuf::from("/tmp")),
                journald: false,
                filter: Some("filter".to_string()),
                log_file_max_size: Some(100),
                log_file_max_files: Some(10),
            },
            verbosity: Verbosity { verbosity: 3, quiet: false },
            config: Config { local_config: PathBuf::from("/tmp/config") },
        };

        let result = cli.apply_cli(builder).unwrap().try_build().unwrap();

        // Now we can assert that the fields of the resulting `PhConfig` match our
        // expectations. Note: You'll need to adjust these assertions based on the actual
        // implementation of `PhConfig`.
        assert_eq!(result.tracing.log_level, "Info".parse().unwrap());
        assert!(!result.tracing.journald);
        assert_eq!(result.tracing.filter, "filter");
        assert_eq!(result.tracing.logs_dir.as_os_str(), "/tmp");
        assert!(result.tracing.persistent);
        assert_eq!(result.tracing.log_file_max_size, 100);
        assert_eq!(result.tracing.log_file_max_files, 10);
        assert!(result.api.enable_api);
        assert!(result.api.enable_metrics);
        assert_eq!(result.api.bind_ip, bind.into());
        assert_eq!(result.source.as_os_str(), "/tmp/config");
    }

    #[test]
    fn test_load_dotenv() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(".env");
        std::fs::write(file_path, "PH_ENV_VAR=TEST").unwrap();

        let config = dir.path().join("phylax.yaml");
        let cli = Cli {
            command: Commands::Node(node::NodeCommand::default()),
            logs: Logs::default(),
            verbosity: Verbosity::default(),
            config: Config { local_config: config, ..Default::default() },
        };
        cli.load_dotenv();
        assert_eq!(env::var("PH_ENV_VAR").unwrap(), "TEST");
    }

    #[test]
    fn test_build_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("phylax.yaml");
        std::fs::write(file_path.clone(), "").unwrap();
        let cli = Cli {
            command: Commands::Node(node::NodeCommand::default()),
            logs: Logs::default(),
            verbosity: Verbosity::default(),
            config: Config::new(file_path),
        };
        let result = cli.build_config();
        assert!(result.is_ok());
    }
}
