use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use crate::{
    activities::{Activity, ActivityContext, ActivityError, FgWorkType},
    events::{Event, EventBuilder, EventType},
    tasks::task::WorkContext,
};
use async_trait::async_trait;
use eyre::Result;
use flume::r#async::RecvStream;

use phylax_monitor::{EvmAlert, EvmAlertResult};
use phylax_tracing::tracing::{debug, error, info, warn};
use serde_json::Value;

use super::evm_watcher::EvmWatcherRes;

/// Enum representing the commands that the EvmAlert activity can execute.
#[derive(Debug, PartialEq)]
pub enum EvmAlertCmd {
    /// Command to run the alert.
    RunAlert { block: Option<u64> },
}

/// Implementation of Activity trait for EvmAlert.
#[async_trait]
impl Activity for EvmAlert {
    type Command = EvmAlertCmd;
    type InternalStream = RecvStream<'static, Self::InternalStreamItem>;
    type InternalStreamItem = EvmAlertResult;

    /// Bootstraps the EvmAlert activity. It builds the alerts. If the alerts revert when built, the
    /// bootstrap will fail. Essentially, it runs `forge build` in the alerts projects.
    async fn bootstrap(&mut self, _ctx: Arc<ActivityContext>) -> Result<()> {
        self.build_alerts()?;
        Ok(())
    }

    /// Decides the next command to execute based on the events.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of events to decide the next command from.
    /// * `_context` - The context of the activity.
    ///
    /// # Returns
    ///
    /// * `Option<Vec<EvmAlertCmd>>` - The next command to execute. If the last event type is
    ///   `NewBlock`, and the event body can be deserialized into `EvmWatcherRes`, it returns a
    ///   `RunAlert` command with the block number from the deserialized info. If the
    ///   deserialization fails, it logs a warning and returns a `RunAlert` command without a block
    ///   number. If the last event type is not `NewBlock`, it returns a `RunAlert` command without
    ///   a block number.
    fn decide(
        &self,
        mut events: Vec<&Event>,
        _context: &WorkContext,
    ) -> std::option::Option<Vec<EvmAlertCmd>> {
        let last_event = events.pop().unwrap();
        if *last_event.get_type() == EventType::NewBlock {
            let block_info: Option<EvmWatcherRes> =
                serde_json::from_value(last_event.get_body_ref().clone()).ok();
            match block_info {
                Some(info) => Some(vec![EvmAlertCmd::RunAlert { block: Some(info.block_number) }]),
                None => {
                    warn!(
                        "Failed to deserialize block info from event body: {}",
                        last_event.get_body_ref()
                    );
                    Some(vec![EvmAlertCmd::RunAlert { block: None }])
                }
            }
        } else {
            Some(vec![EvmAlertCmd::RunAlert { block: None }])
        }
    }

    /// Executes the commands. Run the alert assertions for the block.
    #[allow(irrefutable_let_patterns)]
    async fn do_work(&mut self, mut cmds: Vec<EvmAlertCmd>, ctx: WorkContext) -> Result<()> {
        // Pop the command from the vector of commands
        let activity_context = ctx.activity.clone();
        let context_map: HashMap<String, Value> = ctx.into_map();
        let cmd = cmds.pop().unwrap();
        // Check if the command is RunAlert and execute the alert assertions for the block
        if let EvmAlertCmd::RunAlert { block } = cmd {
            info!(block, "Executing alert assertions for block");
            // Run the tests for the block and get the result
            let res = self.run_tests(context_map).await?;
            debug!(?res, "Alert execution result");
            // Get the duration of the alert execution
            let duration = res.duration.as_secs() as f64;
            // Set the alert duration in the metrics
            activity_context.metrics.set_alert_duration(&activity_context.task_name, duration);
            // Iterate over all exposed values
            res.all_exposed_values.iter().for_each(|exposed_value| {
                // Try to parse the data of the exposed value as a number
                let metric_number_or_err= exposed_value.data.parse();
                // Check if the parsing was successful
                match metric_number_or_err{
                    Ok(val) => {
                        // If successful, set the exported data in the metrics
                        activity_context.metrics
                            .set_exported_data(&activity_context.task_name, &exposed_value.key, exposed_value.chain.as_ref(),  &exposed_value.context, val, exposed_value.block_number);
                    }
                    Err(_) => {
                        // If unsuccessful, log a warning
                        warn!(
                            data=exposed_value.data,
                            key=&exposed_value.key,
                            block=&exposed_value.block_number,
                            chain=%exposed_value.chain,
                            "This exported data is not a number. It will not be exported via the Prometheus endpoint"
                        );
                    }
                }
            });
            // Send the result to the internal transmitter
            self.internal_tx.send(res.clone())?;
            // Check if the setup failed
            if res.failed_setup {
                error!("Alert failed during setUp()");
                return Ok(());
            }
            // Check if the alert is firing
            if res.alert_firing() {
                // If the alert is firing, set the alert firing in the metrics and log a warning
                activity_context.metrics.set_alert_firing(&activity_context.task_name, true);
                warn!(failed_tests = res.failed_tests_pretty(), "Alert firing");
            } else {
                // If the alert is not firing, set the alert not firing in the metrics
                activity_context.metrics.set_alert_firing(&activity_context.task_name, false);
            }
            // Return Ok
            Ok(())
        } else {
            // If the command is not RunAlert, return an error
            Err(ActivityError::UnknownCommand(format!("{cmd}")).into())
        }
    }

    /// Returns the work type of the activity.
    fn work_type(&self) -> FgWorkType {
        FgWorkType::SpawnNonBlocking
    }

    /// Returns the internal stream of the activity.
    async fn internal_stream(&self) -> Option<Self::InternalStream> {
        Some(self.internal_rx.clone().into_stream())
    }

    /// Processes the items from the internal stream and converts them into events.
    /// The internal item here is the result of an alert execution.
    async fn internal_stream_work(
        &mut self,
        item: Self::InternalStreamItem,
        _ctx: Arc<ActivityContext>,
    ) -> Option<Event> {
        Some(item.into())
    }
}

/// Implementation of Display trait for EvmAlertCmd.
impl Display for EvmAlertCmd {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EvmAlertCmd::RunAlert { block } => {
                write!(f, "RunAlert {{ block: {} }}", block.unwrap_or(0))
            }
        }
    }
}

/// Converts an `EvmAlertResult` to an `Event`.
impl From<EvmAlertResult> for Event {
    fn from(res: EvmAlertResult) -> Self {
        let event_type =
            if res.alert_firing() { EventType::AlertFiring } else { EventType::AlertExecution };
        EventBuilder::new()
            .with_type(event_type)
            .with_body(serde_json::to_value(res).unwrap())
            .build()
    }
}
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use crate::mocks::mock_context::mock_activity_context;

    use super::*;
    use ethers_core::types::Chain;
    use phylax_monitor::evm_alert::alert::{AlertState, ExposedValue};
    use serde_json::json;
    use tokio_stream::StreamExt;
    use tracing_test::traced_test;

    fn create_test_alert() -> EvmAlert {
        // Create a new EvmAlert
        let workspace_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = std::path::Path::new(&workspace_root)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("testdata/alerts");
        let (internal_tx, internal_rx) = flume::unbounded();
        EvmAlert {
            state: AlertState::Uninitialized,
            foundry_config_root: path,
            foundry_profile: "default".to_string(),
            internal_tx,
            internal_rx: internal_rx.clone(),
        }
    }

    #[test]

    fn test_evm_alert_result_to_event() {
        // Create an EvmAlertResult instance
        let exposed_value = ExposedValue {
            data: String::from("test_data"),
            key: String::from("test_key"),
            block_number: 42,
            chain: Chain::Mainnet,
            context: String::from("test_context"),
        };

        let evm_alert_result = EvmAlertResult {
            alert_state: AlertState::Firing,
            failed_tests: HashMap::new(),
            failed_setup: true,
            duration: Duration::from_secs(42),
            all_exposed_values: vec![exposed_value],
        };

        // Convert EvmAlertResult to Event
        let event: Event = evm_alert_result.clone().into();

        // Check if the event type is correct
        assert_eq!(
            event.get_type(),
            if evm_alert_result.alert_firing() {
                &EventType::AlertFiring
            } else {
                &EventType::AlertExecution
            }
        );
        // Check if the event body is correct
        assert_eq!(event.get_body_ref(), &serde_json::to_value(evm_alert_result.clone()).unwrap());
        // Assert for values
        assert_eq!(evm_alert_result.alert_state, AlertState::Firing);
        assert!(evm_alert_result.failed_tests.is_empty());
        assert!(evm_alert_result.failed_setup);
        assert_eq!(evm_alert_result.duration, Duration::from_secs(42));
        assert_eq!(evm_alert_result.all_exposed_values[0].data, "test_data");
        assert_eq!(evm_alert_result.all_exposed_values[0].key, "test_key");
        assert_eq!(evm_alert_result.all_exposed_values[0].block_number, 42);
        assert_eq!(evm_alert_result.all_exposed_values[0].chain, Chain::Mainnet);
        assert_eq!(evm_alert_result.all_exposed_values[0].context, "test_context");
    }

    #[test]
    fn test_decide() {
        let evm_alert = EvmAlert {
            state: AlertState::Uninitialized,
            foundry_config_root: "./alerts".parse().unwrap(),
            foundry_profile: "default".to_string(),
            internal_tx: flume::unbounded().0,
            internal_rx: flume::unbounded().1,
        };

        let event = EventBuilder::new()
            .with_type(EventType::NewBlock)
            .with_body(json!({"block_number": 42}))
            .build();

        let events = vec![&event];
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let commands = evm_alert.decide(events, &context);

        assert_eq!(commands, Some(vec![EvmAlertCmd::RunAlert { block: Some(42) }]));
        // Test when block is None
        let event = EventBuilder::new().with_type(EventType::NewBlock).with_body(json!({})).build();

        let events = vec![&event];
        let commands = evm_alert.decide(events, &context);

        assert_eq!(commands, Some(vec![EvmAlertCmd::RunAlert { block: Some(0) }]));

        // Test when event type is not NewBlock
        let event = EventBuilder::new().with_type(EventType::AlertExecution).build();

        let events = vec![&event];
        let commands = evm_alert.decide(events, &context);

        assert_eq!(commands, Some(vec![EvmAlertCmd::RunAlert { block: None }]));
    }
    #[tokio::test]
    async fn test_rpc_do_work() {
        // Create a new EvmAlert
        let mut evm_alert = create_test_alert();
        evm_alert.foundry_profile = "simple".to_owned();
        let mut stream = evm_alert.internal_stream().await.unwrap();

        // Create a new EvmAlertCmd
        let cmds = vec![EvmAlertCmd::RunAlert { block: Some(42) }];

        // Create a new ActivityContext
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());

        // Run the do_work function
        evm_alert.do_work(cmds, context.clone()).await.unwrap();

        let res = stream.next().await.unwrap();
        assert_eq!(res.alert_state, AlertState::Off);
        assert!(!res.failed_setup);
        assert!(res.failed_tests.is_empty());
        assert_eq!(res.all_exposed_values[0].block_number, 19999);
        assert_eq!(res.all_exposed_values[0].key, "chainid");
        assert_eq!(res.all_exposed_values[0].data, "1");
        assert_eq!(res.all_exposed_values[0].chain, Chain::Mainnet);
        assert_eq!(res.all_exposed_values[0].context, "simple/SimpleAlert.t.sol:SimpleAlert");
        assert_eq!(context.activity.metrics.get_alert_firing("mock_task"), 0 as f64);
        assert_eq!(
            context.activity.metrics.get_exported_data_value(
                "mock_task",
                "chainid",
                "mainnet",
                "simple/SimpleAlert.t.sol:SimpleAlert"
            ),
            1_f64
        );
        assert_eq!(
            context.activity.metrics.get_exported_data_last_block(
                "mock_task",
                "chainid",
                "mainnet",
                "simple/SimpleAlert.t.sol:SimpleAlert"
            ),
            19999_i64
        );
    }

    #[tokio::test]
    async fn test_rpc_do_work_firing() {
        // Create a new EvmAlertCmd
        let cmds = vec![EvmAlertCmd::RunAlert { block: Some(42) }];

        let mut evm_alert = create_test_alert();
        evm_alert.foundry_profile = "firing".to_owned();
        let mut stream = evm_alert.internal_stream().await.unwrap();

        let task_name = "firing_alert".to_owned();
        // Create a new ActivityContext
        let context = WorkContext::new(
            mock_activity_context(Some(task_name.clone())),
            EventBuilder::new().build(),
        );
        // Run the do_work function
        evm_alert.do_work(cmds, context.clone()).await.unwrap();

        let failed_tests: HashMap<String, String> = vec![
            ("testBlockNumber()".to_owned(), "No revert info".to_owned()),
            ("testExport()".to_owned(), "EvmError: Revert".to_owned()),
        ]
        .into_iter()
        .collect();

        let res = stream.next().await.unwrap();

        assert_eq!(res.alert_state, AlertState::Firing);
        assert!(!res.failed_setup);
        assert_eq!(res.failed_tests, failed_tests);
        assert!(!res.failed_tests.is_empty());
        assert_eq!(res.all_exposed_values[0].block_number, 19999);
        assert_eq!(res.all_exposed_values[0].key, "chainid");
        assert_eq!(res.all_exposed_values[0].data, "1");
        assert_eq!(res.all_exposed_values[0].chain, Chain::Mainnet);
        assert_eq!(res.all_exposed_values[0].context, "firing/FiringAlert.t.sol:FiringAlert");
        assert_eq!(context.activity.metrics.get_alert_firing(&task_name), 1_f64);
    }
    /// Test the bootstrap function
    #[tokio::test]
    async fn test_bootstrap() {
        let mut evm_alert = create_test_alert();
        let ctx = mock_activity_context(None);
        let result = evm_alert.bootstrap(ctx).await;
        assert!(result.is_ok());
    }

    /// Test the work_type function
    #[test]
    fn test_work_type() {
        let evm_alert = create_test_alert();
        assert_eq!(evm_alert.work_type(), FgWorkType::SpawnNonBlocking);
    }

    /// Test the internal_stream function
    #[tokio::test]
    async fn test_internal_stream() {
        let evm_alert = create_test_alert();
        let result = evm_alert.internal_stream().await;
        assert!(result.is_some());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_rpc_do_work_import_context() {
        // Create a new EvmAlert
        let mut evm_alert = create_test_alert();
        evm_alert.foundry_profile = "import".to_owned();
        let mut stream = evm_alert.internal_stream().await.unwrap();

        // Create a new EvmAlertCmd
        let cmds = vec![EvmAlertCmd::RunAlert { block: Some(42) }];

        // Create a new ActivityContext
        let context = WorkContext::new(
            mock_activity_context(Some("test".to_string())),
            EventBuilder::new().with_origin(123).build(),
        );

        // Run the do_work function
        evm_alert.do_work(cmds, context.clone()).await.unwrap();

        let res = stream.next().await.unwrap();
        assert_eq!(res.alert_state, AlertState::Off);
        assert!(!res.failed_setup);
        assert!(res.failed_tests.is_empty());
    }
}
