use crate::error::PhylaxNodeError;
use phylax_interfaces::{
    error::PhylaxTaskError,
    event::{EventBus, EventType, NodeShutdownCause},
    executors::{TaskExecutor, TaskManager},
};
use phylax_tracing::tracing::{
    self, debug, error, instrument, span, trace, Instrument, Level, Span,
};
use std::time::Duration;
use tokio::{
    pin,
    runtime::Handle,
    select,
    sync::{
        mpsc,
        oneshot::{self, error::RecvError},
    },
    task::JoinHandle,
};

const SHUTDOWN_TIMEOUT_IN_SECS: u64 = 1;

/// Struct with various handles and channels required for managing the lifecycle of tasks within a
/// node.
///
/// This struct is used to encapsulate the necessary components for task execution, internal and
/// external shutdown signaling, and graceful shutdown handling.
///
/// # Fields
///
/// * `task_executor` - The executor responsible for running tasks.
/// * `exit_handle` - A handle for the task that manages the lifecycle, which returns a result
///   indicating success or failure.
/// * `external_shutdown_tx` - An optional sender for signaling an external shutdown with an
///   optional duration for graceful shutdown.
/// * `internal_shutdown_tx` - A sender for signaling an internal shutdown, which can carry a result
///   indicating success or failure.
pub(crate) struct LifecycleHandles {
    pub task_executor: TaskExecutor,
    pub exit_handle: JoinHandle<Result<(), PhylaxNodeError>>,
    pub external_shutdown_tx: Option<oneshot::Sender<Option<Duration>>>,
    pub internal_shutdown_tx: mpsc::UnboundedSender<Result<(), PhylaxTaskError>>,
}

/// Spawns a task which manages a receiver for all shutdown signal types for the node, and returns
/// useful lifecycle handles.
///
/// This function sets up the necessary components to manage the lifecycle of tasks within the node,
/// including task execution, internal and external shutdown signaling, and graceful shutdown
/// handling.
///
/// # Arguments
///
/// * `handle` - A Tokio runtime handle used to spawn tasks.
/// * `node_span` - A tracing span that serves as the parent span for the lifecycle task.
#[instrument(parent = node_span, skip(handle))]
pub fn init_lifecycle_task(
    handle: Handle,
    event_bus: EventBus,
    node_span: &Span,
) -> LifecycleHandles {
    let mut manager = TaskManager::new(handle.clone());
    let task_executor = manager.executor();
    let (external_shutdown_tx, external_shutdown_rx) = oneshot::channel::<Option<Duration>>();
    let (internal_shutdown_tx, internal_shutdown_rx) =
        mpsc::unbounded_channel::<Result<(), PhylaxTaskError>>();
    let node_span = span!(parent: node_span, Level::INFO, "[LifecycleTask]");
    // This handle returns for either a Phylax task panic or from an external shutdown signal
    let exit_handle = handle.spawn(async move {
        pin!(internal_shutdown_rx);
        select! {
            task_panic_error = &mut manager => {
                error!(error = ?task_panic_error, "Node task manager detected a panic within a critical task and is shutting down");
                event_bus.send_unchecked(EventType::NodeShutdown(NodeShutdownCause::SystemError { error_msg: task_panic_error.to_string() } ));
                Err(task_panic_error.into())
            },
            internal_signal = internal_shutdown_rx.recv() => handle_internal_signal(internal_signal, manager, event_bus,),
            external_signal = external_shutdown_rx => handle_external_signal(external_signal, manager, event_bus,)
        }
    }.instrument(node_span.or_current()));

    LifecycleHandles {
        exit_handle,
        task_executor,
        external_shutdown_tx: Some(external_shutdown_tx),
        internal_shutdown_tx,
    }
}

fn handle_internal_signal(
    signal: Option<Result<(), PhylaxTaskError>>,
    manager: TaskManager,
    event_bus: EventBus,
) -> Result<(), PhylaxNodeError> {
    debug!("Received internal shutdown signal");
    match signal {
        // Received an intentional, internal shutdown from an activity runtime that is not
        // associated with an system-level error
        Some(Ok(_)) => {
            trace!("Received an internal activity shutdown signal");
            event_bus.send_unchecked(EventType::NodeShutdown(NodeShutdownCause::ActivitySignal));
            manager.graceful_shutdown_with_timeout(Duration::from_secs(SHUTDOWN_TIMEOUT_IN_SECS));
            Ok(())
        }
        // Received an internal shutdown associated with some system runtime error, this is
        // unintentional
        Some(Err(err)) => {
            error!(error = ?err, "Internal shutdown signal associated with a task error");
            event_bus.send_unchecked(EventType::NodeShutdown(NodeShutdownCause::SystemError {
                error_msg: err.to_string(),
            }));
            manager.graceful_shutdown_with_timeout(Duration::from_secs(SHUTDOWN_TIMEOUT_IN_SECS));
            Ok(())
        }
        // The internal shutdown receiver closed unexpectedly
        None => {
            let error_msg = "Internal shutdown signal receiver closed unexpectedly";
            error!(error_msg);
            event_bus.send_unchecked(EventType::NodeShutdown(NodeShutdownCause::SystemError {
                error_msg: error_msg.to_string(),
            }));
            manager.graceful_shutdown_with_timeout(Duration::from_secs(SHUTDOWN_TIMEOUT_IN_SECS));
            Err(PhylaxNodeError::InternalSignalError)
        }
    }
}

fn handle_external_signal(
    signal: Result<Option<Duration>, RecvError>,
    manager: TaskManager,
    event_bus: EventBus,
) -> Result<(), PhylaxNodeError> {
    debug!("Received an external shutdown signal");
    event_bus.send_unchecked(EventType::NodeShutdown(NodeShutdownCause::ExternalSignal));
    match signal {
        // External shutdown within the specified timeout
        Ok(Some(duration)) => {
            trace!(timeout = ?duration, "Shutting down with timeout");
            if manager.graceful_shutdown_with_timeout(duration) {
                trace!(timeout = ?duration, "Shutdown completed before timeout");
                Ok(())
            } else {
                trace!(timeout = ?duration, "Shutdown process timed out");
                Err(PhylaxNodeError::ShutdownTimeoutError(duration))
            }
        }
        // External shutdown with no specified timeout, but 1 second by default
        Ok(None) => {
            trace!("Shutting down without timeout");
            manager.graceful_shutdown_with_timeout(Duration::from_secs(SHUTDOWN_TIMEOUT_IN_SECS));
            Ok(())
        }
        // The external sender was dropped without sending
        Err(e) => {
            error!(error = ?e, "External shutdown sender dropped without sending");
            manager.graceful_shutdown_with_timeout(Duration::from_secs(SHUTDOWN_TIMEOUT_IN_SECS));
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::FutureExt;
    use phylax_interfaces::executors::{shutdown::Shutdown, TaskManager};
    use tokio::time::Duration;

    pub fn setup_stubs() -> (TaskManager, Shutdown, EventBus) {
        let manager = TaskManager::new(Handle::current());
        let shutdown_handle = manager.executor().on_shutdown_signal().clone();
        let event_bus = EventBus::new();
        (manager, shutdown_handle, event_bus)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handle_internal_signal_ok() {
        let (manager, shutdown_handle, mut event_bus) = setup_stubs();

        let signal = Some(Ok(()));
        let result = handle_internal_signal(signal, manager, event_bus.clone());
        assert!(result.is_ok());
        let shutdown_event_type = event_bus.receive().await.expect("to receive a message").body;
        assert!(matches!(
            shutdown_event_type,
            EventType::NodeShutdown(NodeShutdownCause::ActivitySignal)
        ));
        assert_eq!(shutdown_handle.now_or_never(), Some(()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handle_internal_signal_err() {
        let (manager, shutdown_handle, mut event_bus) = setup_stubs();

        let signal = Some(Err(PhylaxTaskError::MessageCastError("Dummy error".to_string())));
        let result = handle_internal_signal(signal, manager, event_bus.clone());
        assert!(result.is_ok());
        let shutdown_event_type = event_bus.receive().await.expect("to receive a message").body;
        assert!(matches!(
            shutdown_event_type,
            EventType::NodeShutdown(NodeShutdownCause::SystemError { .. })
        ));
        assert_eq!(shutdown_handle.now_or_never(), Some(()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handle_internal_signal_none() {
        let (manager, shutdown_handle, mut event_bus) = setup_stubs();

        let signal = None;
        let result = handle_internal_signal(signal, manager, event_bus.clone());
        assert!(result.is_err());
        let shutdown_event_type = event_bus.receive().await.expect("to receive a message").body;
        assert!(matches!(
            shutdown_event_type,
            EventType::NodeShutdown(NodeShutdownCause::SystemError { .. })
        ));
        assert_eq!(shutdown_handle.now_or_never(), Some(()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handle_external_signal_no_duration() {
        let (manager, shutdown_handle, mut event_bus) = setup_stubs();

        let signal = Ok(None);
        let result = handle_external_signal(signal, manager, event_bus.clone());
        assert!(result.is_ok());
        let shutdown_event_type = event_bus.receive().await.expect("to receive a message").body;
        assert!(matches!(
            shutdown_event_type,
            EventType::NodeShutdown(NodeShutdownCause::ExternalSignal)
        ));
        assert_eq!(shutdown_handle.now_or_never(), Some(()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handle_external_signal_with_duration_in_time() {
        let (manager, shutdown_handle, mut event_bus) = setup_stubs();

        let signal = Ok(Some(Duration::from_secs(2)));
        let result = handle_external_signal(signal, manager, event_bus.clone());
        assert!(result.is_ok());
        let shutdown_event_type = event_bus.receive().await.expect("to receive a message").body;
        assert!(matches!(
            shutdown_event_type,
            EventType::NodeShutdown(NodeShutdownCause::ExternalSignal)
        ));
        assert_eq!(shutdown_handle.now_or_never(), Some(()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handle_external_signal_none() {
        let (manager, shutdown_handle, mut event_bus) = setup_stubs();
        // Since we can't create a RecvError, we need to create it this way
        let (tx, rx) = oneshot::channel();
        drop(tx);
        let err_signal = rx.await;

        let result = handle_external_signal(err_signal, manager, event_bus.clone());
        assert!(result.is_err());
        let shutdown_event_type = event_bus.receive().await.expect("to receive a message").body;
        assert!(matches!(
            shutdown_event_type,
            EventType::NodeShutdown(NodeShutdownCause::ExternalSignal)
        ));
        assert_eq!(shutdown_handle.now_or_never(), Some(()));
    }
}
