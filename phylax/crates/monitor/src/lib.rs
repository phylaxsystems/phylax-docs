mod error;
pub mod evm_alert;
mod evm_watcher;

// Re-export the modules
pub use error::{EvmAlertError, EvmWatcherError};
pub use evm_alert::alert::{EvmAlert, EvmAlertResult};
pub use evm_watcher::{EvmWatcher, Providers};
