use crate::{error::EventBufferError, events::Event};
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use std::sync::Arc;

type BufferProd = Producer<Event, Arc<SharedRb<Event, Vec<std::mem::MaybeUninit<Event>>>>>;
type BufferCon = Consumer<Event, Arc<SharedRb<Event, Vec<std::mem::MaybeUninit<Event>>>>>;

/// `EventBuffer` is a buffer that temporarily stores events that a [`super::task::Task`] is
/// subscribed to. These events are not immediately processed if the Task is currently executing a
/// foreground work. The buffer allows for the deferred processing of these events, ensuring that no
/// event is missed even when the Task is busy with other operations.
/// It uses [`ringbuf`] under the hood
pub struct EventBuffer {
    /// The [`ringbuf::Producer`]
    prod: BufferProd,
    /// The [`ringbuf::Consumer`]
    cons: BufferCon,
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBuffer {
    /// The size of the buffer.
    // TODO: benchmark this number
    pub const CAPACITY: usize = 100;
    /// Creates a new `EventBuffer` with a capacity defined by `CAPACITY`.
    pub fn new() -> Self {
        let rb = HeapRb::<Event>::new(Self::CAPACITY);
        let (prod, cons) = rb.split();
        Self { prod, cons }
    }

    /// Pushes an event into the buffer.
    /// Returns an `EventBufferError` if the buffer is full.
    pub fn push(&mut self, event: Event) -> Result<(), EventBufferError> {
        match self.prod.push(event) {
            Ok(_) => Ok(()),
            Err(err) => Err(EventBufferError::PushError(err)),
        }
    }

    /// Returns an iterator over the events in the buffer.
    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.cons.iter()
    }

    /// Pops an event from the buffer.
    /// Returns `None` if the buffer is empty.
    pub fn pop(&mut self) -> Option<Event> {
        self.cons.pop()
    }

    /// Checks if the buffer is full.
    /// Returns `true` if the buffer is full, `false` otherwise.
    pub fn is_full(&self) -> bool {
        self.cons.is_full()
    }

    /// Calculates the free space in the buffer as a percentage of the total capacity.
    /// Returns the free space as a percentage.
    pub fn free_percentage(&self) -> usize {
        let free_space = Self::CAPACITY - self.cons.len();

        (free_space * 100) / Self::CAPACITY
    }

    /// Clears the buffer, removing all events.
    pub fn clear(&mut self) {
        self.cons.clear();
    }

    pub fn len(&self) -> usize {
        self.cons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cons.is_empty()
    }
}

/// Implements conversion from `EventBuffer` to `Vec<&Event>`.
/// This allows for easy conversion of the buffer into a vector of events.
impl<'a> From<&'a EventBuffer> for Vec<&'a Event> {
    fn from(buffer: &'a EventBuffer) -> Self {
        buffer.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{Event, EventBuilder}; // Assuming Event is defined in the crate

    /// Test the creation of a new EventBuffer
    #[test]
    fn test_new() {
        let buffer = EventBuffer::new();
        assert_eq!(buffer.free_percentage(), 100);
    }

    /// Test pushing an event into the EventBuffer
    #[test]
    fn test_push() {
        let mut buffer = EventBuffer::new();
        let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
        assert!(buffer.push(event).is_ok());
    }

    /// Test popping an event from the EventBuffer
    #[test]
    fn test_pop() {
        let mut buffer = EventBuffer::new();
        let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
        buffer.push(event).unwrap();
        assert!(buffer.pop().is_some());
    }

    /// Test checking if the EventBuffer is full
    #[test]
    fn test_is_full() {
        let mut buffer = EventBuffer::new();
        for _ in 0..EventBuffer::CAPACITY {
            let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
            buffer.push(event).unwrap();
        }
        assert!(buffer.is_full());
    }

    /// Test clearing the EventBuffer
    #[test]
    fn test_clear() {
        let mut buffer = EventBuffer::new();
        let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
        buffer.push(event).unwrap();
        buffer.clear();
        assert!(buffer.pop().is_none());
    }

    /// Test the conversion from EventBuffer to Vec<&Event>
    #[test]
    fn test_conversion() {
        let mut buffer = EventBuffer::new();
        let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
        buffer.push(event).unwrap();
        let vec: Vec<&Event> = Vec::from(&buffer);
        assert_eq!(vec.len(), 1);
    }

    #[test]
    fn test_push_full() {
        let mut buffer = EventBuffer::new();
        for _ in 0..EventBuffer::CAPACITY {
            let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
            buffer.push(event).unwrap();
        }
        let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
        assert!(buffer.push(event).is_err());
    }

    /// Test calculating the free space in the EventBuffer
    #[test]
    fn test_free_percentage() {
        let mut buffer = EventBuffer::new();
        let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
        buffer.push(event).unwrap();
        assert_eq!(buffer.free_percentage(), 99);
    }

    /// Test iterating over the events in the EventBuffer
    #[test]
    fn test_iter() {
        let mut buffer = EventBuffer::new();
        let event = EventBuilder::new().with_origin(73).build(); // Assuming Event has a new() method
        buffer.push(event).unwrap();
        let mut iter = buffer.iter();
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }
}
