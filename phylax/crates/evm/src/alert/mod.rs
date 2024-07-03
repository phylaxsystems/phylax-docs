/// The config for the evm alert.
pub mod config;

/// The typed error for the evm alert.
pub mod error;

use crate::watcher::EvmWatcherMessage;

use phylax_runner::{AssertionOutcome, Backend, DataAccesses, PhylaxRunner};

use phylax_interfaces::{
    activity::{
        Activity, MetricKind, MonitorConfig, MonitorKind, PointValue, TimeSeriesConfig,
        Visualization,
    },
    context::{ActivityContext, HealthSeverity},
    error::PhylaxError,
    message::MessageDetails,
};
use phylax_tracing::tracing::{self, info, instrument, trace};

use error::EvmAlertError;

use std::time::{Duration, Instant};

/// The EvmAlert is the activity for receiving [`EvmWatcherMessage`],
/// executing assertions,
/// and returning the [`EvmAlertMessage`]
#[derive(Debug)]
pub struct EvmAlert {
    /// Identifying name configured for the alert.
    name: String,
    /// Runner used for executing assertions.
    runner: PhylaxRunner,
}

impl EvmAlert {
    const TIME_TO_EXECUTE_ASSERTIONS: &'static str = "Time To Execute Assertions";

    /// Sends time it took to detect the block to the monitor
    fn send_time_to_execute_assertions(
        context: &ActivityContext,
        duration: Duration,
    ) -> Result<(), PhylaxError> {
        context.health_monitors.get(Self::TIME_TO_EXECUTE_ASSERTIONS).unwrap().send_u64(
            duration
                .as_millis()
                .try_into()
                .map_err(|err| PhylaxError::UnrecoverableError(Box::new(err)))?,
            HealthSeverity::Healthy,
        );

        Ok(())
    }
}

impl Activity for EvmAlert {
    const FLAVOR_NAME: &'static str = "EvmAlert";

    type Input = EvmWatcherMessage;
    type Output = EvmAlertMessage;

    const MONITORS: &'static [MonitorConfig] = &[MonitorConfig {
        name: Self::TIME_TO_EXECUTE_ASSERTIONS,
        description: "The time it took to execute the assertions.",
        display: Some(Visualization::TimeSeries(TimeSeriesConfig {
            x_axis_name: "Block Number",
            y_axis_name: "Time To Execute",
        })),
        category: MonitorKind::Health {
            unhealthy_message: "Assertions took longer than normal to execute.",
        },
        metric_type: Some(MetricKind::Gauge),
        unit: "seconds",
        value_type: PointValue::Uint(0),
    }];

    #[instrument(skip_all, fields(activity = Self::FLAVOR_NAME, input = ?_input))]
    async fn process_message(
        &mut self,
        _input: Option<(EvmWatcherMessage, MessageDetails)>,
        context: ActivityContext,
    ) -> Result<Option<EvmAlertMessage>, PhylaxError> {
        info!(name = self.name, "Running EvmAlert");

        let db = context.state_registry.get::<Backend>().ok_or_else(|| {
            EvmAlertError::TypeNotFoundInStateRegistry("Backend".to_owned()).into()
        })?;

        // Execute assertions and submit the duration to a monitor.
        let timer = Instant::now();
        let (assertion_outcome, access_diff) = self.runner.execute_assertions(db.clone());
        Self::send_time_to_execute_assertions(&context, timer.elapsed())?;

        // Apply access diff to track data accesses globally
        let data_accesses = context.state_registry.get::<DataAccesses>().ok_or_else(|| {
            EvmAlertError::TypeNotFoundInStateRegistry("DataAccesses".to_owned()).into()
        })?;

        trace!(?access_diff, "Applying access diff");
        data_accesses.apply_access_diff(access_diff);

        Ok(Some(EvmAlertMessage { name: self.name.to_owned(), assertion_outcome }))
    }
}

///Output Message of the [`EvmAlert`]
#[derive(Debug)]
pub struct EvmAlertMessage {
    /// The name of the Alert which output this message.
    pub name: String,
    /// The outcome of the assertion run.
    pub assertion_outcome: AssertionOutcome,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_evm_alert() {
    use forge_cmd::FilterArgs;
    use phylax_runner::{Compile, PhylaxRunner};
    use phylax_test_utils::{get_alert_context, get_alerts_build_args};
    use regex::Regex;

    let filter = FilterArgs {
        contract_pattern: Some(Regex::new("SimpleAlert").unwrap()),
        ..Default::default()
    };

    let mut alert = EvmAlert {
        name: "simple_alert".to_owned(),
        runner: PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap(),
    };

    let outcome =
        alert.process_message(None, get_alert_context::<EvmAlert>()).await.unwrap().unwrap();

    assert_eq!(outcome.name, "simple_alert");
    assert_eq!(outcome.assertion_outcome.failures.len(), 0);
    assert_eq!(outcome.assertion_outcome.exports.len(), 2);
}
