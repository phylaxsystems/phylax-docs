use crate::{error::PhylaxNodeError, tasks::monitor::MonitorWatcher};
use itertools::{concat, Either, Itertools};
use kanal::AsyncReceiver;
use phylax_interfaces::{
    activity::DynActivity, error::PhylaxTaskError, event::EventBus, executors::TaskExecutor,
    message::BoxedMessage, state_registry::StateRegistry,
};
use phylax_tasks::{
    activity::ActivityCell,
    task::{ActivityCategory, Task, TaskChannels},
};
use phylax_tracing::tracing::{self, error, instrument, trace, Span};
use std::sync::{atomic::AtomicU64, Arc};

use tokio::sync::mpsc;

/// Validates activity definitions and configures them into tasks.
///
/// This function initializes tasks from a list of activities, assigns them to the appropriate
/// categories (watchers, actions, alerts), and sets up the necessary message passing channels
/// between them. It ensures that tasks are correctly linked based on their input and output
/// types, facilitating the flow of messages through the system.
///
/// # Arguments
///
/// * `activities` - A vector of boxed dynamic [`Activity`] to be configured into a [`Task`] vector.
/// * `executor` - The task executor responsible for running the tasks.
/// * `internal_shutdown` - An unbounded sender for signaling internal shutdowns.
/// * `event_bus` - A placeholder for the event bus, currently unused.
/// * `message_id_counter` - An atomic counter for generating unique message IDs.
/// * `span` - The node tracing span to be used as the parent of the tasks' spans.
/// * `state_registry` - A state registry for the node.
///
/// # Returns
///
/// A result containing a vector of Arc-wrapped tasks if successful, or a `PhylaxNodeError` if an
/// error occurs.
///
/// # Errors
///
/// This function can return a `PhylaxNodeError` if task initialization fails or if any tasks are
/// found to have invalid definitions, such as lacking connections to their [`Activity`]'s `Input`
/// or `Output` types.
///
/// # Examples
///
/// ```
/// # use phylax_interfaces::{activity::{Activity, DynActivity}, context::ActivityContext, executors::TaskManager, message::MessageDetails, event::EventBus, error::PhylaxError, state_registry::StateRegistryBuilder};
/// # use phylax_tasks::task::Task;
/// # use tokio::sync::mpsc;
/// # use phylax_tracing::tracing::Span;
/// # use phylax_tasks::activity::ActivityCell;
///
/// # use std::sync::Arc;
/// # use std::sync::atomic::AtomicU64;
/// # tokio_test::block_on(async {
///
///
/// pub struct MyActivity;
/// pub struct MyActivityMessage;
/// impl Activity for MyActivity {
///     const FLAVOR_NAME: &'static str = "my_activity_flavor";
///     type Input = MyActivityMessage;
///     type Output = MyActivityMessage;
///
///     async fn process_message(&mut self, input: Option<(MyActivityMessage, MessageDetails)>, context: ActivityContext) -> Result<Option<MyActivityMessage>, PhylaxError> {
///         Ok(None)
///     }
/// }
///
/// let activity1 = MyActivity;
/// let activity2 = MyActivity;
///
/// let (internal_shutdown_sender, _) = mpsc::unbounded_channel();
///
/// let activities = vec![Box::new(ActivityCell::new("activity_1", activity1)) as Box<dyn DynActivity>,
///     Box::new(ActivityCell::new("activity_2", activity2)) as Box<dyn DynActivity>];
///
/// let tasks = phylax_core::utils::configure_tasks(
///     activities,
///     TaskManager::current().executor(),
///     internal_shutdown_sender,
///     EventBus::new(),
///     Arc::new(AtomicU64::new(0)),
///     &Span::none(),
///     StateRegistryBuilder::default().build(),
/// ).await.expect("Task configuration failed");
///
/// # });
/// ```
#[instrument(parent = span, skip_all)]
pub async fn configure_tasks(
    activities: Vec<Box<dyn DynActivity>>,
    executor: TaskExecutor,
    internal_shutdown: mpsc::UnboundedSender<Result<(), PhylaxTaskError>>,
    event_bus: EventBus,
    message_id_counter: Arc<AtomicU64>,
    span: &Span,
    state_registry: StateRegistry,
) -> Result<Vec<Arc<Task>>, PhylaxNodeError> {
    let state_registry = Arc::new(state_registry);
    let mut all_activity_receivers = Vec::new();
    let mut all_health_receivers = Vec::new();
    let mut all_monitor_configs = Vec::new();
    let tasks_and_channels: Vec<(Task, Option<AsyncReceiver<BoxedMessage>>)> = activities
        .into_iter()
        .map(|activity| {
            all_monitor_configs.extend(activity.get_monitor_configs());

            // TODO: Handle component display info and merge with notification component display
            // info
            let (task, task_channels, _component_display_info) = Task::init(
                executor.clone(),
                event_bus.clone(),
                message_id_counter.clone(),
                activity,
                internal_shutdown.clone(),
                span,
                state_registry.clone(),
            );

            // Aggregate all activity and health receivers
            let TaskChannels { activity_receivers, health_receivers, maybe_outbox_receiver } =
                task_channels;
            all_activity_receivers.extend(activity_receivers);
            all_health_receivers.extend(health_receivers);

            (task, maybe_outbox_receiver)
        })
        .collect();

    let monitor_watcher = ActivityCell::new(
        "monitor_watcher",
        MonitorWatcher::new(
            all_activity_receivers,
            all_health_receivers,
            all_monitor_configs.as_slice(),
        ),
    );
    let (monitor_task, _, _) = Task::init(
        executor.clone(),
        event_bus.clone(),
        message_id_counter.clone(),
        Box::new(monitor_watcher),
        internal_shutdown.clone(),
        &Span::none(),
        state_registry,
    );

    trace!("Initialized tasks and output receivers");

    // Separate watchers, actions, and alerts
    let (watchers, mut actions_alerts): (Vec<_>, Vec<_>) = tasks_and_channels
        .into_iter()
        .partition(|(task, _)| matches!(task.details.task_category, ActivityCategory::Watcher));
    for (watcher, maybe_outbox_receiver) in watchers.iter() {
        let output_type = watcher.activity.lock().await.get_output_type_id();
        if let Some(outbox) = &maybe_outbox_receiver {
            for (alert_or_action, _) in actions_alerts.iter_mut() {
                if output_type == alert_or_action.activity.lock().await.get_input_type_id() {
                    alert_or_action.input_receivers.push(outbox.clone())
                }
            }
        }
    }

    let (mut actions, mut alerts): (Vec<_>, Vec<_>) = actions_alerts
        .into_iter()
        .partition(|(task, _)| matches!(task.details.task_category, ActivityCategory::Action));

    for i in 0..alerts.len() {
        // Use mutable slices of the alerts array so that we may modify an element while
        // borrowing another.
        let (left_slice, mid_right_slice) = alerts.split_at_mut(i);
        let (mid_slice, right_slice) = mid_right_slice.split_at_mut(1);
        let (alert, maybe_outbox_receiver) = &mut mid_slice[0];

        let output_type = alert.activity.lock().await.get_output_type_id();
        if let Some(outbox) = &maybe_outbox_receiver {
            for (actions, _) in actions.iter_mut() {
                if output_type == actions.activity.lock().await.get_input_type_id() {
                    actions.input_receivers.push(outbox.clone())
                }
            }
            // Check every alert against every other alert
            // WARNING: This can cause cycles in the Task graph!
            for (other_alert, _) in left_slice {
                let other_input_type = other_alert.activity.lock().await.get_input_type_id();
                if output_type == other_input_type {
                    other_alert.input_receivers.push(outbox.clone())
                }
            }
            for (other_alert, _) in right_slice {
                let other_input_type = other_alert.activity.lock().await.get_input_type_id();
                if output_type == other_input_type {
                    other_alert.input_receivers.push(outbox.clone())
                }
            }
        }
    }

    // Check if there are any Watchers without outputs, Alerts without inputs or outputs,
    // or Actions without inputs
    let mut tasks = validate_tasks(watchers, alerts, actions)?;
    tasks.push(monitor_task);

    let thread_safe_tasks = tasks.into_iter().map(Arc::new).collect();
    Ok(thread_safe_tasks)
}

/// Validates the tasks by checking for any Watchers without outputs, Alerts without inputs or
/// outputs, or Actions without inputs.
///
/// This function separates activities with valid connections to their inputs and outputs from
/// activities that are missing connections to their inputs and outputs. It partitions the given
/// tasks into broken and configured tasks based on their connectivity and returns an error if any
/// tasks are found to be broken.
///
/// # Arguments
///
/// * `watchers` - A vector of tuples containing Watcher tasks and their optional output receivers.
/// * `alerts` - A vector of tuples containing Alert tasks and their optional output receivers.
/// * `actions` - A vector of tuples containing Action tasks and their optional output receivers.
///
/// # Returns
///
/// * `Result<Vec<Task>, PhylaxNodeError>` - A result containing a vector of valid tasks wrapped in
///   `Arc` if all tasks are valid, or a `PhylaxNodeError` if any tasks are found to be broken.
///
/// # Errors
///
/// This function returns a `PhylaxNodeError::ActivityConfigurationError` if any Watchers, Alerts,
/// or Actions are found to be broken due to missing connections to their inputs or outputs.
#[instrument(skip_all)]
fn validate_tasks(
    watchers: Vec<(Task, Option<AsyncReceiver<BoxedMessage>>)>,
    alerts: Vec<(Task, Option<AsyncReceiver<BoxedMessage>>)>,
    actions: Vec<(Task, Option<AsyncReceiver<BoxedMessage>>)>,
) -> Result<Vec<Task>, PhylaxNodeError> {
    // Separate activities with valid connections to their inputs and outputs from activities
    // that are missing connections to their inputs and outputs.
    let (broken_watchers, configured_watchers): (Vec<_>, Vec<_>) =
        watchers.into_iter().partition_map(|(watcher, mut maybe_outbox_receiver)| {
            if let Some(outbox) = maybe_outbox_receiver.take() {
                if outbox.receiver_count() <= 1 {
                    // Broken due to no receivers for the output type
                    Either::Left(watcher)
                } else {
                    Either::Right(watcher)
                }
            } else {
                // This should be unreachable as Watchers should always have a valid output
                // receiver channel
                unreachable!(
                    "A Watcher was detected without a valid output message receiver channel"
                );
            }
        });
    if !broken_watchers.is_empty() {
        let watcher_names = broken_watchers
            .iter()
            .map(|task| format!("{:?}", task.details))
            .collect::<Vec<_>>()
            .join(", ");
        error!(
            watchers = watcher_names,
            "Some Watchers could not be configured with any valid output Activities"
        );
        return Err(PhylaxNodeError::ActivityConfigurationError(format!("Unable to configure some Watchers because of lack of valid output Activities. Broken Watchers: {}", watcher_names)));
    }
    trace!("Initialized all watchers");

    let (broken_alerts, configured_alerts): (Vec<_>, Vec<_>) =
        alerts.into_iter().partition_map(|(alert, mut maybe_outbox_receiver)| {
            if let Some(outbox) = maybe_outbox_receiver.take() {
                if outbox.receiver_count() <= 1 || alert.input_receivers.is_empty() {
                    // Broken due to no receivers for the output type or no input receivers
                    // being added to the task
                    Either::Left(alert)
                } else {
                    Either::Right(alert)
                }
            } else {
                // This should be unreachable as Alerts should always have a valid output
                // receiver channel
                unreachable!(
                    "An Alert was detected without a valid output message receiver channel"
                );
            }
        });
    if !broken_alerts.is_empty() {
        let alert_names = broken_alerts
            .iter()
            .map(|task| format!("{:?}", task.details))
            .collect::<Vec<_>>()
            .join(", ");
        error!(
            alerts = alert_names,
            "Some Alerts could not be configured with any valid input or output Activities"
        );
        return Err(PhylaxNodeError::ActivityConfigurationError(format!("Unable to configure some Alerts because of lack of valid input or output Activities. Broken Alerts: {}", alert_names)));
    }
    trace!("Initialized all alerts");

    let (broken_actions, configured_actions): (Vec<_>, Vec<_>) = actions
        .into_iter()
        .map(|(action, _)| action)
        .partition(|action| action.input_receivers.is_empty());
    if !broken_actions.is_empty() {
        let action_names = broken_actions
            .iter()
            .map(|task| format!("{:?}", task.details))
            .collect::<Vec<_>>()
            .join(", ");
        error!(
            actions = action_names,
            "Some Actions could not be configured with any valid input Activities"
        );
        return Err(PhylaxNodeError::ActivityConfigurationError(format!("Unable to configure some Actions because of lack of valid input Activities. Broken Actions: {}", action_names)));
    }
    trace!("Initialized all actions");

    let valid_tasks = concat([configured_watchers, configured_alerts, configured_actions]);
    Ok(valid_tasks)
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports, dead_code)]
    use crate::builder::PhylaxBuilder;

    use super::*;
    use futures::pin_mut;
    use phylax_interfaces::{
        activity::Activity, context::ActivityContext, error::PhylaxError, executors::TaskManager,
        message::MessageDetails, state_registry::StateRegistryBuilder,
    };
    use phylax_test_utils::mock_activity;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use tokio::{runtime::Runtime, select, task::JoinHandle};

    pub struct MockWatcherMessage {
        pub magic_num: u64,
    }
    pub struct MockWatcher;
    mock_activity!(MockWatcher, (), MockWatcherMessage, self _input _context {
        tokio::time::sleep(Duration::from_millis(2000)).await;
        Ok(Some(MockWatcherMessage {
            magic_num: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("SystemTime failed")
                .as_secs(),
        }))
    });

    pub struct MockAction;
    mock_activity!(MockAction, MockWatcherMessage, (), self _input _context { Ok(None) });

    pub struct MockAlert;
    mock_activity!(MockAlert, MockWatcherMessage, MockWatcherMessage, self _input _context { Ok(None) });

    async fn configure_default_tasks(
        custom_activities: Vec<Box<dyn DynActivity>>,
    ) -> Result<Vec<Arc<Task>>, PhylaxNodeError> {
        let manager = TaskManager::current();
        let exec = manager.executor();
        let event_bus = EventBus::new();
        let (sender, _) = mpsc::unbounded_channel::<Result<(), PhylaxTaskError>>();
        configure_tasks(
            custom_activities,
            exec.clone(),
            sender,
            event_bus,
            Arc::new(AtomicU64::new(0)),
            &Span::none(),
            StateRegistryBuilder::default().build(),
        )
        .await
    }

    #[tokio::test]
    async fn test_configure_tasks_watcher_and_action_no_alerts() {
        // Watcher -> Action
        let builder = PhylaxBuilder::new()
            .with_custom_activity("WatcherName", MockWatcher)
            .with_custom_activity("ActionName", MockAction);

        let tasks = configure_default_tasks(builder.custom_activities)
            .await
            .expect("Should not encounter task validation errors");

        assert_eq!(tasks.len(), 3); // Two custom activity tasks and a monitor task
        assert!(tasks
            .iter()
            .any(|task| matches!(task.details.task_category, ActivityCategory::Watcher)));
        assert!(tasks
            .iter()
            .any(|task| matches!(task.details.task_category, ActivityCategory::Action)));
        for task in tasks {
            if task.details.task_name == "monitor_watcher" {
                continue;
            }
            let task_type = &task.details.task_category;
            match task_type {
                ActivityCategory::Watcher => {
                    assert_eq!(task.input_receivers.len(), 0);
                    assert!(task.output_sender.is_some());
                    if let Some(outbox) = &task.output_sender {
                        assert_eq!(outbox.receiver_count(), 1);
                    }
                }
                ActivityCategory::Action => {
                    assert_eq!(task.input_receivers.len(), 1);
                    assert!(task.output_sender.is_none());
                }
                _ => panic!("This should never be reached"),
            }
        }
    }

    #[tokio::test]
    async fn test_configure_tasks_multiple_connections() {
        // Watcher -> Alert1 -> Action
        // Watcher -> Alert2 -> Action
        // Alert1 -> Alert2
        // Alert2 -> Alert1
        // Watcher -> Action
        let builder = PhylaxBuilder::new()
            .with_custom_activity("WatcherName", MockWatcher)
            .with_custom_activity("AlertName1", MockAlert)
            .with_custom_activity("AlertName2", MockAlert)
            .with_custom_activity("ActionName", MockAction);

        let tasks = configure_default_tasks(builder.custom_activities)
            .await
            .expect("Should not encounter task validation errors");
        // This should create 4 tasks with an excessive amount of shared message channels, as well
        // as a fifth monitor task
        assert_eq!(tasks.len(), 5);
        assert!(tasks
            .iter()
            .any(|task| matches!(task.details.task_category, ActivityCategory::Watcher)));
        assert!(tasks
            .iter()
            .any(|task| matches!(task.details.task_category, ActivityCategory::Alert)));
        assert!(tasks
            .iter()
            .any(|task| matches!(task.details.task_category, ActivityCategory::Action)));
        for task in tasks {
            if task.details.task_name == "monitor_watcher" {
                continue;
            }
            let task_type = &task.details.task_category;
            match task_type {
                ActivityCategory::Watcher => {
                    assert_eq!(task.input_receivers.len(), 0);
                    assert!(task.output_sender.is_some());
                    if let Some(outbox) = &task.output_sender {
                        assert_eq!(outbox.receiver_count(), 3);
                    }
                }
                ActivityCategory::Alert => {
                    assert_eq!(task.input_receivers.len(), 2);
                    assert!(task.output_sender.is_some());
                }
                ActivityCategory::Action => {
                    assert_eq!(task.input_receivers.len(), 3);
                    assert!(task.output_sender.is_none());
                }
            }
        }
    }

    #[tokio::test]
    async fn test_configure_tasks_circular_alerts() {
        // Alert1 -> Alert2
        // Alert2 -> Alert1
        let builder = PhylaxBuilder::new()
            .with_custom_activity("AlertName1", MockAlert)
            .with_custom_activity("AlertName2", MockAlert);

        let tasks = configure_default_tasks(builder.custom_activities)
            .await
            .expect("Should not encounter task validation errors");
        // This should create two alert tasks that simply send each other messages if they have
        // received one, as well as a monitor task
        assert_eq!(tasks.len(), 3);
        assert!(tasks.iter().all(|task| {
            // Skip monitor task
            if task.details.task_name == "monitor_watcher" {
                return true;
            }

            matches!(task.details.task_category, ActivityCategory::Alert)
        }));
        for task in tasks {
            if task.details.task_name == "monitor_watcher" {
                continue;
            }
            let task_type = &task.details.task_category;
            match task_type {
                ActivityCategory::Alert => {
                    assert_eq!(task.input_receivers.len(), 1);
                    assert!(task.output_sender.is_some());
                }
                _ => panic!("This should never be reached"),
            }
        }
    }

    #[tokio::test]
    async fn test_configure_tasks_fails_for_dangling_watcher() {
        // Watcher -> x
        let builder = PhylaxBuilder::new().with_custom_activity("WatcherName", MockWatcher);

        let cfg_res = configure_default_tasks(builder.custom_activities).await;
        assert!(matches!(cfg_res, Err(PhylaxNodeError::ActivityConfigurationError(_))));
    }

    #[tokio::test]
    async fn test_configure_tasks_fails_for_dangling_action() {
        // x -> Action
        let builder = PhylaxBuilder::new().with_custom_activity("ActionName", MockAction);

        let cfg_res = configure_default_tasks(builder.custom_activities).await;
        assert!(matches!(cfg_res, Err(PhylaxNodeError::ActivityConfigurationError(_))));
    }

    #[tokio::test]
    async fn test_configure_tasks_fails_for_dangling_alerts() {
        // x -> Alert -> x

        let builder = PhylaxBuilder::new().with_custom_activity("AlertName", MockAlert);

        let cfg_res = configure_default_tasks(builder.custom_activities).await;

        // Should fail when missing inputs and outputs
        assert!(matches!(cfg_res, Err(PhylaxNodeError::ActivityConfigurationError(_))));

        // x -> Alert -> Action
        let builder = PhylaxBuilder::new()
            .with_custom_activity("ActionName", MockAction)
            .with_custom_activity("AlertName", MockAlert);
        let cfg_res = configure_default_tasks(builder.custom_activities).await;
        // Should fail when missing only inputs
        assert!(matches!(cfg_res, Err(PhylaxNodeError::ActivityConfigurationError(_))));

        // Watcher -> Alert -> x
        let builder = PhylaxBuilder::new()
            .with_custom_activity("WatcherName", MockWatcher {})
            .with_custom_activity("AlertName", MockAlert {});
        let cfg_res = configure_default_tasks(builder.custom_activities).await;
        // Should fail when missing only outputs
        assert!(matches!(cfg_res, Err(PhylaxNodeError::ActivityConfigurationError(_))));
    }

    #[tokio::test]
    async fn test_monitor_watcher_task_is_created() {
        let tasks = configure_default_tasks(Vec::new())
            .await
            .expect("Should not encounter task validation errors");
        // The system should configure a monitor watcher task by default
        assert!(tasks.iter().all(|task| {
            // Skip monitor task
            if task.details.task_name == "monitor_watcher" {
                return true;
            }

            matches!(task.details.task_category, ActivityCategory::Watcher)
        }));
    }
}
