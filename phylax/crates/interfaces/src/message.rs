use std::{
    any::Any,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};
/// MessageDetails are the metadata for an incoming message which can be consumed by an
/// [`Activity`].
#[derive(Debug)]
pub struct MessageDetails {
    /// The name of the task that emitted the message.
    pub origin: String,
    /// The unique id of the message, incremented from 0 starting at Phylax node startup.
    pub id: u64,
    /// The UNIX timestamp in microseconds at which the message was emitted.
    pub timestamp: u128,
}

/// The type-erased message struct used in the Phylax runtime, emitted and consumed by tasks.
///
/// This struct is used when needing to wrap the associated input or output types of an
/// [`Activity`] trait for functions used by the [`Task`].
pub struct BoxedMessage {
    pub inner: Box<dyn Any + Send>,
    pub details: MessageDetails,
}

impl BoxedMessage {
    /// Create a new boxed message decorated with
    pub fn new(
        inner: Box<dyn Any + Send>,
        origin: String,
        message_counter: &Arc<AtomicU64>,
    ) -> Result<Self, SystemTimeError> {
        let details = MessageDetails {
            origin,
            id: message_counter.fetch_add(1, Ordering::Relaxed),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros(),
        };

        Ok(Self { inner, details })
    }
}
