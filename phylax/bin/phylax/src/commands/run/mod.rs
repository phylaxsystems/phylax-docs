//! Main node command for launching a node

use clap::Parser;
use phylax_core::{
    args::no_args::NoArgs,
    builder::PhylaxBuilder,
    dirs::{DataDirPath, MaybePlatformPath},
    utils::parse_socket_address,
    version,
};
use phylax_tracing::tracing;
use reth_cli_runner::CliContext;
use std::{ffi::OsString, fmt, future::Future, net::SocketAddr, path::PathBuf};

/// Start the node
#[derive(Debug, Parser)]
pub struct RunCommand<Ext: clap::Args + fmt::Debug = NoArgs> {
    /// The path to the data dir for all phylax files and subdirectories.
    ///
    /// Defaults to the OS-specific data directory:
    ///
    /// - Linux: `$XDG_DATA_HOME/phylax/` or `$HOME/.local/share/phylax/`
    /// - Windows: `{FOLDERID_RoamingAppData}/phylax/`
    /// - macOS: `$HOME/Library/Application Support/phylax/`
    #[arg(long, value_name = "DATA_DIR", verbatim_doc_comment, default_value_t)]
    pub datadir: MaybePlatformPath<DataDirPath>,

    /// The path to the configuration file to use.
    #[arg(long, value_name = "FILE", verbatim_doc_comment)]
    pub config: Option<PathBuf>,

    /// The path to the root dir
    #[arg(long, value_name = "FILE", verbatim_doc_comment)]
    pub root: Option<PathBuf>,

    /// Enable Prometheus metrics.
    ///
    /// The metrics will be served at the given interface and port.
    #[arg(long, value_name = "SOCKET", value_parser = parse_socket_address, help_heading = "Metrics")]
    pub metrics: Option<SocketAddr>,

    /// Additional cli arguments
    #[command(flatten, next_help_heading = "Extension")]
    pub ext: Ext,
}

impl RunCommand {
    /// Parsers only the default CLI arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Parsers only the default [RunCommand] arguments from the given iterator
    pub fn try_parse_args_from<I, T>(itr: I) -> Result<Self, clap::error::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Self::try_parse_from(itr)
    }
}

impl<Ext: clap::Args + fmt::Debug> RunCommand<Ext> {
    /// Launches the node
    ///
    /// This transforms the node command into a node config and launches the node using the given
    /// closure.
    pub async fn execute<L, Fut>(self, _ctx: CliContext, launcher: L) -> eyre::Result<()>
    where
        L: FnOnce(PhylaxBuilder, Ext) -> Fut,
        Fut: Future<Output = eyre::Result<()>>,
    {
        tracing::info!(target: "phylax::cli", version = ?version::SHORT_VERSION, "Starting phylax");

        let Self { datadir: _datadir, config: _config, metrics: _metrics, ext, root } = self;

        let root = root.unwrap_or(std::env::current_dir()?);

        let builder = PhylaxBuilder::new().with_root(root);
        launcher(builder, ext).await
    }
}
