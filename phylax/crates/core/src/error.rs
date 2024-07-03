use phylax_interfaces::executors::PanickedTaskError;
use phylax_tracing::error::PhylaxTracingError;
use std::time::Duration;
use tokio::{sync::oneshot::error::RecvError, task::JoinError};

/// An error kind which covers potential error conversions from logic in the
/// cache or underlying state provider implementations.
#[derive(thiserror::Error, Debug)]
pub enum PhylaxNodeError {
    /// Errors that occur during the initialization of tracing, logging, or
    /// metrics provider initialization.
    #[error("TaskManager JoinHandle had a problem resolving: {0:?}")]
    TelemetryInitError(#[from] PhylaxTracingError),
    /// Error type associated with the join handle of a
    /// long running TaskManager problem.
    #[error("TaskManager JoinHandle had a problem resolving: {0:?}")]
    ManagerJoinError(#[from] JoinError),
    /// Configuration problem during task setup, such as being
    /// unable to build a graph of valid tasks
    #[error("Activity configuration error: {0:?}")]
    ActivityConfigurationError(String),
    /// Error type associated when there is a task panic
    /// inside one of the the critical tasks
    #[error("Critical task encountered a panic: {0:?}")]
    TaskCriticalPanic(#[from] PanickedTaskError),
    /// Task bootstrap problem that was triggered prior to the task launch
    #[error("Task bootstrap error: {0:?}")]
    TaskBootstrapError(String),
    /// The shutdown signal receiver closed unexpectedly, possibly from being closed manually or
    /// from all senders being closed separately.
    #[error("The internal shutdown signal receiver was closed unexpectedly")]
    InternalSignalError,
    /// Error type associated with issues that happens when a node has been signaled to
    /// shutdown externally, such as when the external shutdown sender is dropped unexpectedly.
    #[error("Unexpected shutdown signal error: {0:?}")]
    ExternalSignalError(#[from] RecvError),
    /// Attempted to await the exit future when it was detached from the node and was replaced
    /// with an option.
    #[error("Attempted to await the node exit future when it was no longer available.")]
    MissingExitFutError,
    /// This error is returned when not all tasks shutdown in the requested
    /// time that was sent to the shutdown signal. Task names aren't returned
    /// when this error is triggered at the moment.
    #[error("Some tasks did not shutdown within timeout duration: {0:?}")]
    ShutdownTimeoutError(Duration),
}
