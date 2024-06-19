use color_eyre::Report;
use phylax_config::EvmAlertConfig;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvmWatcherError {
    #[error("The Watcher can't identify the Provider")]
    UnknownProvider,
    #[error("The Block stream closed unexpectedly")]
    BlockStreamClosed,
    #[error("The watcher found a block hash, but can't retrieve it again. Probably a re-org of the chain")]
    UnknownHash,
    #[error("RPC endpoint can't be parsed into a URL: {0}")]
    InvalidRpc(#[from] url::ParseError),
}

#[derive(Debug, Error)]
pub enum EvmAlertError {
    #[error("Failed to create an Alert from config {0:?}")]
    CreateFromConfig(EvmAlertConfig),
    #[error("Failed to build Alert due to {source}")]
    BuildAlert { source: Report },
    #[error("Failed to compile the Solidity contracts due to {errors:?}")]
    CompileAlert { errors: Vec<String> },
    #[error("Failed to find the alert source files")]
    SourceNotFound,
}
