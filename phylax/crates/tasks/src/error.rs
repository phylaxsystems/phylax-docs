use std::time::SystemTimeError;

use kanal::SendError;
use phylax_interfaces::error::PhylaxError;
use tokio::task::JoinError;

/// An error kind which covers potential error conversions from logic in the
/// cache or underlying state provider implementations.
#[derive(thiserror::Error, Debug)]
pub enum PhylaxTaskError {
    /// An error variant which can be returned by the activity bootstrap during startup.
    #[error("There was an error during activity bootstrap: {0:?}")]
    ActivitySpawnError(#[source] PhylaxError),
    /// An error variant which can be returned by the activity cleanup during shutdown.
    #[error("There was an error during activity cleanup: {0:?}")]
    ActivityShutdownError(#[source] PhylaxError),
    /// An error variant which is returned by the user implemented activity message processing
    /// function, which can represent both unrecoverable and recoverable errors.
    #[error("There was an internal logic error returned by the completed task execution: {0:?}")]
    ActivityProcessError(#[source] PhylaxError),
    /// Returned when attempting to send a message to an outbox, but instead encountering an
    /// unexpected error from using the channel.
    #[error("There was an internal logic error returned by the completed task execution: {0:?}")]
    OutboxSendError(#[from] SendError),
    /// Returned when attempting to cast an incoming message into the configured associated
    /// input type, but failing to do so. The runtime should never allow this to happen.
    #[error("Could not cast message to concrete type: {0:?}")]
    MessageCastError(String),
    /// An error variant which arises when awaiting a task's join handle which has panicked, and
    /// is unrecoverable unless specifically from a task cancellation.
    #[error("There was a task level error from a join handle that interrupted the task execution: {0:?}")]
    JoinHandleError(#[from] JoinError),
    /// An error variant when attempting to box a message produced by an activity, and represents
    /// a problem with accessing the system time.
    #[error("An error occured while getting the system time during message creation: {0:?}")]
    MessageCreationError(#[from] SystemTimeError),
}
