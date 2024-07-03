use phylax_interfaces::{
    activity::{Activity, DynActivity, Label, MonitorConfig},
    context::ActivityContext,
    error::PhylaxTaskError,
    message::{BoxedMessage, MessageDetails},
};
use std::{
    any::{self, Any, TypeId},
    sync::{atomic::AtomicU64, Arc},
};
use tokio_util::sync::ReusableBoxFuture;

/// The wrapper struct that erases associated types and allows for object safe dyn Activities.
///
/// `ActivityCell` is the wrapper tuple struct for [`Activity`] traits, and which implements the
/// [`DynActivity`] trait. This wrapper is required to allow the trait object version of Activities
/// to be handled by the Phylax system instead of the [`Activity`] trait with associated types.
pub struct ActivityCell<T: Activity> {
    task_name: String,
    inner: T,
}

impl<T: Activity> ActivityCell<T> {
    pub fn new(name: impl Into<String>, activity: T) -> Self {
        Self { task_name: name.into(), inner: activity }
    }
}

impl<T: Activity> DynActivity for ActivityCell<T> {
    fn bootstrap(
        &mut self,
        context: ActivityContext,
    ) -> ReusableBoxFuture<Result<(), PhylaxTaskError>> {
        let fut = self.inner.bootstrap(context);
        ReusableBoxFuture::new(
            async move { fut.await.map_err(PhylaxTaskError::ActivitySpawnError) },
        )
    }

    /// The concrete implementation for the process_message wrapper function.
    ///
    /// This function not only returns the results from the inner [`Activity`] trait's
    /// `process_message` function, but also consumes a `BoxedMessage` that is expected to be of
    /// the associated `Input` type of the Activity and attempts to downcast it to the correct type.
    ///
    /// It also decorates the output message of the `process_message` associated `Output` type with
    /// the correct [`MessageDetails`] metadata to be used by other tasks, and wraps both structs
    ///  in the [`BoxedMessage`] struct.
    ///
    /// This function will panic if it consumes a [`BoxedMessage`] with an inner message type
    /// that does not match the associated `Input` type of the inner Activity of the
    /// [`ActivityCell`],
    fn process_message(
        &mut self,
        input: Option<BoxedMessage>,
        context: ActivityContext,
        message_counter: Arc<AtomicU64>,
    ) -> ReusableBoxFuture<Result<Option<BoxedMessage>, PhylaxTaskError>> {
        // If a message triggered this work, then downcast the message into the expected type for
        // the Activity's input.
        let maybe_destructured_msg: Option<(T::Input, MessageDetails)> =
            if let Some(boxed_msg) = input {
                let details = boxed_msg.details;
                // Attempt to convert the inner boxed message into the associated input type
                let input_type = match boxed_msg.inner.downcast::<T::Input>() {
                    Ok(inner) => inner,
                    Err(_) => {
                        // A message should never fail to convert to the correct input type - this
                        // means that the runtime incorrectly configured the
                        // input receivers for an Activity.
                        let type_name = any::type_name::<T::Input>();
                        let error_message = format!(
                            "{} activity could not cast message to: {}",
                            T::FLAVOR_NAME,
                            type_name
                        );
                        return ReusableBoxFuture::new(async {
                            Err(PhylaxTaskError::MessageCastError(error_message))
                        });
                    }
                };
                // Convert to the structure expected by the `Activity` trait
                Some((*input_type, details))
            } else {
                None
            };

        let result_fut = self.inner.process_message(maybe_destructured_msg, context);
        let task_name = self.task_name.clone();
        ReusableBoxFuture::new(async move {
            // Await the result and try to extract the message or return a task error.
            let maybe_message = result_fut.await.map_err(PhylaxTaskError::ActivityProcessError)?;

            if let Some(message) = maybe_message {
                // If there is a message, box it to erase its generic type, and then return it.
                let boxed_message_body = Box::new(message) as Box<dyn Any + Send>;
                BoxedMessage::new(boxed_message_body, task_name, &message_counter)
                    .map(Some)
                    .map_err(PhylaxTaskError::from)
            } else {
                // Otherwise, simply return None.
                Ok(None)
            }
        })
    }

    /// The concrete implementation for the cleanup wrapper function.
    fn cleanup(
        &mut self,
        context: ActivityContext,
    ) -> ReusableBoxFuture<Result<(), PhylaxTaskError>> {
        let fut = self.inner.cleanup(context);
        ReusableBoxFuture::new(
            async move { fut.await.map_err(PhylaxTaskError::ActivityShutdownError) },
        )
    }

    /// A wrapper function for returning the TypeId of the underlying associated `Input` type
    /// for the Activity trait
    ///
    /// This is used primarily during the task runtime setup.
    fn get_input_type_id(&self) -> TypeId {
        TypeId::of::<T::Input>()
    }

    /// A wrapper function for returning the [`TypeId`] of the underlying associated `Output` type
    /// for the Activity trait
    ///
    /// This is used primarily during the task runtime setup.
    fn get_output_type_id(&self) -> TypeId {
        TypeId::of::<T::Output>()
    }

    /// The name associated with the activity type definition - the same for all tasks constructed
    /// of this type.
    fn get_activity_flavor(&self) -> &'static str {
        T::FLAVOR_NAME
    }

    /// The unqiue name associated with this individual instance of an activity type.
    fn get_activity_name(&self) -> String {
        self.task_name.clone()
    }

    /// Returns a static slice of [`MonitorConfig`] defined by the activity for tracking, displaying
    /// and exporting information about the behavior and health of the activity.
    fn get_monitor_configs(&self) -> &'static [MonitorConfig] {
        T::MONITORS
    }

    /// Returns a static slice of [`Label`]s defined by the activity for defining activity metadata,
    /// for the GUI to use for decoration.
    fn get_labels(&self) -> &'static [Label] {
        T::LABELS
    }
}
