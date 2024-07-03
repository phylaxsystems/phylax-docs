use crate::{error::PhylaxNodeError, exit::PhylaxExitFuture};
use phylax_interfaces::{event::EventBus, executors::TaskExecutor};
use phylax_tracing::tracing::Span;
use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
use tokio::sync::oneshot;

/// The launched and configured Phylax Node.
///
/// The `PhylaxNode` is the main interface for interacting with the lifecycle
/// and state of the executable.
///
/// # Examples
///
/// ```
/// use phylax_core::{node::PhylaxNode, builder::PhylaxBuilder};
///
/// # tokio_test::block_on(async {
/// // You can poll for the system to reach completion directly
/// let mut phylax: PhylaxNode = PhylaxBuilder::new()
///     .launch().await
///     .expect("Should launch successfully");
///
///
/// phylax.shutdown_signal.take().unwrap().send(None).unwrap();
///
/// // Wait until the node shutdown is completed or the timeout is reached
///
/// phylax.exit_future.unwrap().await.expect("Should exit successfully");
/// # });
/// ```
#[derive(Debug)]
pub struct PhylaxNode {
    /// State components of the node that aren't related to its high level lifecycle.
    pub components: PhylaxComponents,
    /// Channel for signalling to gracefully shut down the node, with an
    /// optional duration for how long to wait before signalling an error
    /// to the exit future.
    pub shutdown_signal: Option<oneshot::Sender<Option<Duration>>>,
    /// The exit future of the node.
    pub exit_future: Option<PhylaxExitFuture>,
}

impl PhylaxNode {
    /// Waits for the node to exit, if it was configured to exit.
    pub async fn wait_for_node_exit(self) -> Result<(), PhylaxNodeError> {
        match self.exit_future {
            Some(fut) => fut.await,
            None => Err(PhylaxNodeError::MissingExitFutError),
        }
    }
}

/// State components related to the execution of the launched Phylax node.
/// State components related to the execution of the launched Phylax node.
///
/// The `PhylaxComponents` struct encapsulates various non-lifecycle related interfaces
/// for interacting with the node's state.
///
/// # Fields
///
/// * `task_executor` - The [`TaskExecutor`] responsible for managing and running tasks within the
///   node.
/// * `node_span` - The tracing span associated with the node, used for logging and tracing
///   purposes.
/// * `message_id_counter` - An atomic counter used to generate unique message IDs.
/// * `event_bus` - A placeholder for the event bus, which is currently not implemented.
#[derive(Debug)]
pub struct PhylaxComponents {
    pub task_executor: TaskExecutor,
    pub node_span: Span,
    pub message_id_counter: Arc<AtomicU64>,
    pub event_bus: EventBus,
    //pub config: PhylaxConfig,
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::builder::PhylaxBuilder;
    use core::panic;
    use futures::{pin_mut, FutureExt};
    use kanal::{AsyncReceiver, AsyncSender, ReceiveError};
    use phylax_interfaces::{
        activity::{Activity, Label},
        context::ActivityContext,
        error::PhylaxError,
        executors::TaskManager,
        message::MessageDetails,
    };
    use phylax_test_utils::mock_activity;
    use std::{
        cell::RefCell,
        sync::{Arc, RwLock},
        time::Duration,
    };
    use tokio::{
        runtime::{Handle, Runtime},
        select,
        sync::mpsc,
        task::JoinHandle,
    };

    pub struct MockWatcherMessage;
    pub struct MockAction;
    mock_activity!(MockAction, MockWatcherMessage, (), self _input _context {
        Ok(None)
    });

    pub struct MockPanicWatcher {
        receiver: AsyncReceiver<()>,
    }

    mock_activity!(MockPanicWatcher, (), MockWatcherMessage, self _input _context {
        loop {
            select! {
                _ = tokio::time::sleep(Duration::from_millis(1000)) => {},
                _ = self.receiver.recv() => panic!("Intentional panic to end task!"),
            }
        }
    });

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn test_spawned_tasks_run_in_parallel() {
        pub struct MockBlockingWatcher {
            sender: mpsc::Sender<()>,
            receiver: Arc<RwLock<mpsc::Receiver<()>>>,
        }

        // Implement in full to avoid issues with self.sender dropping unexpectedly
        impl Activity for MockBlockingWatcher {
            const FLAVOR_NAME: &'static str = "BlockingWatcher";
            const LABELS: &'static [Label] = &[Label { key: "env", value: "test" }];
            type Input = ();
            type Output = MockWatcherMessage;

            async fn process_message(
                &mut self,
                _input: Option<(Self::Input, MessageDetails)>,
                _context: ActivityContext,
            ) -> Result<Option<Self::Output>, PhylaxError> {
                // This future should never yield, meaning a single threaded runtime or non-parallel
                // concurrent execution of tasks will never execute another task
                let sender = self.sender.try_send(());
                assert!(sender.is_ok());
                // Blocks entire tokio thread
                let receiver = self.receiver.write();
                assert!(receiver.is_ok());
                let mut receiver = receiver.unwrap();
                loop {
                    match &mut receiver.try_recv() {
                        Ok(_) | Err(mpsc::error::TryRecvError::Disconnected) => break,
                        Err(_) => std::thread::sleep(Duration::from_millis(2000)),
                    }
                }
                panic!("System shutdown");
            }
        }

        let (tx1, rx1) = mpsc::channel(4);
        let (tx2, rx2) = mpsc::channel(4);
        let mut node = PhylaxBuilder::new()
            .with_custom_activity(
                "WatcherName3",
                MockBlockingWatcher { sender: tx1, receiver: Arc::new(RwLock::new(rx2)) },
            )
            .with_custom_activity(
                "WatcherName2",
                MockBlockingWatcher { sender: tx2, receiver: Arc::new(RwLock::new(rx1)) },
            )
            .with_custom_activity("ActionName", MockAction)
            .launch()
            .await
            .expect("Should launch node");

        let node_result = node.exit_future.take().expect("exit future should exist").await;
        if let Err(PhylaxNodeError::TaskCriticalPanic(panicked_task_error)) = node_result {
            assert_eq!(
                panicked_task_error.to_string(),
                "Critical task `Watcher` panicked: `System shutdown`"
            );
        } else {
            panic!("Should have reached a task panic error");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_node_errors_on_task_panic() {
        let (tx1, rx1) = kanal::unbounded_async::<()>();

        let builder = PhylaxBuilder::new()
            .with_custom_activity("WatcherName", MockPanicWatcher { receiver: rx1 })
            .with_custom_activity("ActionName", MockAction);

        let mut node = builder.launch().await.expect("Node should build");
        let exit = node.exit_future.take().expect("Exit future should be attached");
        let maybe_exited = futures::future::maybe_done(exit);

        pin_mut!(maybe_exited);

        assert!(maybe_exited.as_mut().take_output().is_none());
        tx1.send(()).await.expect("Should safely send a panic signal");
        maybe_exited.as_mut().await;
        let output = maybe_exited.as_mut().take_output();
        assert!(output.is_some());
        assert!(output.unwrap().is_err())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 6)]
    async fn test_node_external_shutdown_exits_oks() {
        pub struct MockWatcher;
        mock_activity!(MockWatcher, (), MockWatcherMessage, self _input _context {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            Ok(None)
        });

        let mut node = PhylaxBuilder::new()
            .with_custom_activity("WatcherName", MockWatcher)
            .with_custom_activity("ActionName", MockAction)
            .launch()
            .await
            .expect("Node should build");

        let mut exit = node.exit_future.take().expect("Exit future should be attached").boxed();

        if (&mut exit).now_or_never().is_some() {
            panic!("Should not have exited yet");
        } else {
            let maybe_signal = node.shutdown_signal.take();
            let _ = maybe_signal
                .map(|signal| signal.send(None))
                .expect("Should safely send a panic signal");
            let exit_res = exit.await;
            assert!(exit_res.is_ok());
        }
        drop(node)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 6)]
    async fn test_node_external_shutdown_with_duration_errors() {
        pub struct MockWatcher;
        mock_activity!(MockWatcher, (), MockWatcherMessage, self _input _context {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            Ok(None)
        });

        let mut node = PhylaxBuilder::new()
            .with_custom_activity("WatcherName", MockWatcher)
            .with_custom_activity("ActionName", MockAction)
            .launch()
            .await
            .expect("Node should build");

        let mut exit = node.exit_future.take().expect("Exit future should be attached").boxed();

        if (&mut exit).now_or_never().is_some() {
            panic!("Should not have exited yet");
        } else {
            let maybe_signal = node.shutdown_signal.take();
            let _ = maybe_signal
                .map(|signal| signal.send(Some(Duration::from_micros(1))))
                .expect("Should safely send a panic signal");
            let exit_res = exit.await;
            assert!(matches!(exit_res, Err(PhylaxNodeError::ShutdownTimeoutError(_))));
        }
        drop(node)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 6)]
    async fn test_messages_passes_through_watcher_alert_actions() {
        pub struct MockWatcherMessage {
            first_message: String,
        }
        pub struct MockWatcher;
        mock_activity!(MockWatcher, (), MockWatcherMessage, self _input _context {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            Ok(Some(MockWatcherMessage { first_message: "Welcome to the".to_string() }))
        });

        pub struct MockAlertMessage {
            first_message: String,
            second_message: String,
        }

        pub struct MockAlert;
        mock_activity!(MockAlert, MockWatcherMessage, MockAlertMessage, self input _context {
            let (message, _) = input.expect("There should be an associated message");
            let new_message = MockAlertMessage {
                first_message: message.first_message,
                second_message: "Thunderdome".to_string(),
            };
            Ok(Some(new_message))
        });

        pub struct MockAction {
            sender: AsyncSender<MockAlertMessage>,
        }

        mock_activity!(MockAction, MockAlertMessage, (), self input _context {
            let (message, _) = input.expect("There should be an associated message");
            self.sender
                .send(message)
                .await
                .expect("The sender should successfully send the message");
            Ok(None)
        });

        let (tx1, rx1) = kanal::unbounded_async::<MockAlertMessage>();

        let builder = PhylaxBuilder::new()
            .with_custom_activity("WatcherName", MockWatcher)
            .with_custom_activity("AlertName", MockAlert)
            .with_custom_activity("ActionName", MockAction { sender: tx1 });
        let node = builder.launch().await.expect("Node should build");

        let message = rx1.recv().await.expect("Should safely receive the message");
        assert_eq!(message.first_message, "Welcome to the");
        assert_eq!(message.second_message, "Thunderdome");
        drop(node)
    }
}
