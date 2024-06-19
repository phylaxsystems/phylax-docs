use thiserror::Error;

use crate::events::Event;

#[derive(Error, Debug)]
pub enum EventBusError {
    #[error("The Event Bus encountered an error when sending an event")]
    SendError(#[from] tokio::sync::broadcast::error::SendError<Event>),
    #[error("The Event Bus encountered an error when reading an event")]
    RecvError(#[from] tokio::sync::broadcast::error::RecvError),
}

#[derive(Debug, Error)]
pub enum EventBufferError {
    #[error("The event buffer errored when pushing the event: {0}")]
    PushError(Event),
}
