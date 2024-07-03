use clap::{Parser, Subcommand};
use eyre::Result;
use phylax_core::{
    args::{log::LogArgs, no_args::NoArgs},
    builder::PhylaxBuilder,
    version::{LONG_VERSION, SHORT_VERSION},
};
use phylax_tracing::{error::PhylaxTracingError, trace::layers::FileWorkerGuard};
use reth_cli_runner::CliRunner;
use std::{ffi::OsString, fmt, future::Future};

use crate::commands::run;

/// The main Phylax cli interface.
///
/// This is the entrypoint to the executable.
#[derive(Debug, Parser)]
#[command(author, version = SHORT_VERSION, long_version = LONG_VERSION, about = "Phylax", long_about = None)]
pub struct Cli<Ext: clap::Args + fmt::Debug = NoArgs> {
    /// The command to run
    #[command(subcommand)]
    command: Commands<Ext>,

    #[command(flatten)]
    logs: LogArgs,
}

impl Cli {
    /// Parses only the default CLI arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Parsers only the default CLI arguments from the given iterator
    pub fn try_parse_args_from<I, T>(itr: I) -> Result<Self, clap::error::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Cli::try_parse_from(itr)
    }
}

impl<Ext: clap::Args + fmt::Debug> Cli<Ext> {
    pub fn run<L, Fut>(self, launcher: L) -> Result<()>
    where
        L: FnOnce(PhylaxBuilder, Ext) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let _guard = self.init_tracing()?;

        let runner = CliRunner::default();
        match self.command {
            Commands::Run(command) => {
                runner.run_command_until_exit(|ctx| command.execute(ctx, launcher))
            }
        }
    }

    /// Initializes tracing with the configured options.
    ///
    /// If file logging is enabled, this function returns a guard that must be kept alive to ensure
    /// that all logs are flushed to disk.
    pub fn init_tracing(&self) -> Result<Option<FileWorkerGuard>, PhylaxTracingError> {
        self.logs.init_tracing()
    }
}

/// Commands to be executed
#[derive(Debug, Subcommand)]
pub enum Commands<Ext: clap::Args + fmt::Debug = NoArgs> {
    /// Start phylax
    #[command(name = "run")]
    Run(run::RunCommand<Ext>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use phylax_core::args::log::ColorMode;

    #[test]
    fn parse_color_mode() {
        let reth = Cli::try_parse_args_from(["phylax", "run", "--color", "always"]).unwrap();
        assert_eq!(reth.logs.color, ColorMode::Always);
    }

    /// Tests that the help message is parsed correctly. This ensures that clap args are configured
    /// correctly and no conflicts are introduced via attributes that would result in a panic at
    /// runtime
    #[test]
    fn test_parse_help_all_subcommands() {
        let reth = Cli::<NoArgs>::command();
        for sub_command in reth.get_subcommands() {
            let err = Cli::try_parse_args_from(["phylax", sub_command.get_name(), "--help"])
                .err()
                .unwrap_or_else(|| {
                    panic!("Failed to parse help message {}", sub_command.get_name())
                });

            // --help is treated as error, but
            // > Not a true "error" as it means --help or similar was used. The help message will be sent to stdout.
            assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
        }
    }

    /// Tests that the log directory is parsed correctly. It's always tied to the specific chain's
    /// name
    #[test]
    fn parse_logs_path() {
        let phylax = Cli::try_parse_args_from(["phylax", "run"]).unwrap();
        let log_dir: phylax_core::dirs::PlatformPath<phylax_core::dirs::LogsDir> =
            phylax.logs.log_file_directory;
        assert!(log_dir.as_ref().ends_with("phylax/logs"), "{log_dir:?}");
    }

    #[test]
    fn parse_env_filter_directives() {
        let temp_dir = tempfile::tempdir().unwrap();

        std::env::set_var("RUST_LOG", "info,evm=debug");
        let phylax = Cli::try_parse_args_from([
            "phylax",
            "run",
            "--datadir",
            temp_dir.path().to_str().unwrap(),
            "--log.file.filter",
            "debug,net=trace",
        ])
        .unwrap();
        assert!(phylax.run(|_, _| async move { Ok(()) }).is_ok());
    }
}
