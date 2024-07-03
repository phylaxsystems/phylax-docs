/// The config for the [`Evmwatcher`]
pub mod config;

/// Typed Error for [`EvmWatcher`]
pub mod error;

use crate::{utils::duration_since_epoch_timestamp, watcher::error::EvmWatcherError};
use alloy_chains::Chain;
use alloy_provider::Provider;
use alloy_rpc_types_eth::{Block, BlockTransactionsKind};

use futures_util::{stream::iter, StreamExt};

use phylax_interfaces::{
    activity::{
        Activity, MetricKind, MonitorConfig, MonitorKind, PointValue, TimeSeriesConfig,
        Visualization,
    },
    context::{ActivityContext, HealthSeverity},
    error::PhylaxError,
    message::MessageDetails,
};
use phylax_tracing::tracing::{self, info, instrument};

use foundry_common::provider::RetryProvider;
use phylax_runner::{Backend, DataAccesses};

use std::time::{Duration, Instant, SystemTime};

/// EvmWatcher, emits a message on every new block
#[derive(Debug)]
pub struct EvmWatcher {
    ///Provider for fetching block data via rpc
    provider: RetryProvider,
    /// Endpoint for loading data accesses
    url: String,
    /// Chain targeted by this watcher
    chain: Chain,
    /// Backend for loading data accesses
    db: Backend,
}

impl EvmWatcher {
    const TIME_TO_FETCH_STATE: &'static str = "Time To Fetch State";
    const TIME_TO_DETECT_BLOCK: &'static str = "Time To Detect Block";
    const BLOCK_DETECTION_STATUS: &'static str = "Block Detection Status";
    const STATE_FETCHING_STATUS: &'static str = "State Fetching Status";

    /// Awaits the next block and returns the full block data
    #[instrument(skip_all, fields(chain = ?self.chain))]
    async fn watch_for_block(&self) -> Option<Block> {
        let poller = self.provider.watch_blocks().await.ok()?;

        let mut stream = poller.into_stream().flat_map(iter).take(1);

        if let Some(block_hash) = stream.next().await {
            self.provider.get_block(block_hash.into(), BlockTransactionsKind::Hashes).await.ok()?
        } else {
            None
        }
    }

    ///Loads the necessary data accesses for the network for the target block.
    #[instrument(skip_all, fields(block_num, chain = ?self.chain))]
    async fn load_data_accesses(
        &self,
        context: &ActivityContext,
        block_num: u64,
    ) -> Result<(), PhylaxError> {
        let data_access_tracker =
            context.state_registry.get::<DataAccesses>().ok_or_else(|| {
                EvmWatcherError::TypeNotFoundInStateRegistry("DataAccesses".to_owned()).into()
            })?;

        let health_severity = if data_access_tracker
            .load_data_accesses(&self.db, self.chain, block_num, self.url.clone())
            .is_ok()
        {
            HealthSeverity::Healthy
        } else {
            HealthSeverity::Warning
        };

        context
            .health_monitors
            .get(Self::STATE_FETCHING_STATUS)
            .ok_or_else(|| {
                EvmWatcherError::MonitorNotFound(Self::STATE_FETCHING_STATUS.to_string()).into()
            })?
            .send_u64(0, health_severity);

        Ok(())
    }

    /// Sends time it took to detect the block to the monitor
    fn send_time_to_detect_block(
        context: &ActivityContext,
        block: &Block,
    ) -> Result<(), PhylaxError> {
        let detection_delay: u64 =
            duration_since_epoch_timestamp(block.header.timestamp, SystemTime::now())
                .as_millis()
                .try_into()
                .map_err(|err| PhylaxError::UnrecoverableError(Box::new(err)))?;

        //TODO: validate detection delays
        let health_severity = if detection_delay < 2_500 {
            HealthSeverity::Healthy
        } else if detection_delay < 5_000 {
            HealthSeverity::Warning
        } else {
            HealthSeverity::Critical
        };

        context
            .health_monitors
            .get(Self::TIME_TO_DETECT_BLOCK)
            .ok_or_else(|| {
                EvmWatcherError::MonitorNotFound(Self::TIME_TO_DETECT_BLOCK.to_string()).into()
            })?
            .send_u64(detection_delay, health_severity);
        Ok(())
    }

    /// Sends time to fetch state to the monitor
    fn send_time_to_fetch_state(
        context: &ActivityContext,
        duration: Duration,
    ) -> Result<(), PhylaxError> {
        let millis = duration
            .as_millis()
            .try_into()
            .map_err(|err| PhylaxError::UnrecoverableError(Box::new(err)))?;

        context
            .health_monitors
            .get(Self::TIME_TO_FETCH_STATE)
            .ok_or_else(|| {
                EvmWatcherError::MonitorNotFound(Self::TIME_TO_FETCH_STATE.to_string()).into()
            })?
            .send_u64(millis, HealthSeverity::Healthy);
        Ok(())
    }

    /// Sends the block detection status to the monitor
    fn send_block_detection_status(
        context: &ActivityContext,
        status: HealthSeverity,
    ) -> Result<(), PhylaxError> {
        context
            .health_monitors
            .get(Self::BLOCK_DETECTION_STATUS)
            .ok_or_else(|| {
                EvmWatcherError::MonitorNotFound(Self::BLOCK_DETECTION_STATUS.to_string()).into()
            })?
            .send_u64(0, status);
        Ok(())
    }
}

impl Activity for EvmWatcher {
    const FLAVOR_NAME: &'static str = "EvmWatcher";

    type Input = ();
    type Output = EvmWatcherMessage;

    const MONITORS: &'static [MonitorConfig] = &[
        MonitorConfig {
            name: Self::TIME_TO_FETCH_STATE,
            description: "The time it took to fetch the predicted state for the new block.",
            display: Some(Visualization::TimeSeries(TimeSeriesConfig {
                x_axis_name: "Block Number",
                y_axis_name: "Time To Fetch",
            })),
            category: MonitorKind::Health{unhealthy_message:"State took longer than normal to receive."},
            metric_type: Some(MetricKind::Gauge),
            unit: "ms",
            value_type:  PointValue::Uint(0)
        },
        MonitorConfig {
            name: Self::STATE_FETCHING_STATUS,
            description: "Is state being loaded without error?",
            display: None,
            category: MonitorKind::Health{unhealthy_message: "Failed to fetch state for new block."},
            metric_type: Some(MetricKind::Gauge),
            unit: "",
            value_type:  PointValue::Uint(0)
        },
        MonitorConfig {
            name: Self::TIME_TO_DETECT_BLOCK,
            description: "The difference between the time the block was produced, and the time the node detected the block.",
            display: Some(Visualization::TimeSeries(TimeSeriesConfig {
                x_axis_name: "Block Number",
                y_axis_name: "Time To Detect",
            })),
            category: MonitorKind::Health{unhealthy_message: "Time to detect block exceeded acceptable threshold."},
            metric_type: Some(MetricKind::Gauge),
            unit: "ms",
            value_type:  PointValue::Uint(0)
        },
        MonitorConfig {
            name: Self::BLOCK_DETECTION_STATUS,
            description: "Are blocks successfully being detected?",
            display: None,
            category: MonitorKind::Health{unhealthy_message: "Failed to fetch new block data"},
            metric_type: None,
            unit: "",
            value_type:  PointValue::Uint(0)
        }
    ];

    #[instrument(skip_all, fields(activity = Self::FLAVOR_NAME, chain = ?self.chain))]
    async fn process_message(
        &mut self,
        _input: Option<((), MessageDetails)>,
        context: ActivityContext,
    ) -> Result<Option<EvmWatcherMessage>, PhylaxError> {
        info!(chain = ?self.chain, "Running EvmWatcher");

        //Wait for block and report
        //Retry until successful
        let block_num;
        loop {
            if let Some(block) = self.watch_for_block().await {
                if let Some(ok_block_num) = block.header.number {
                    block_num = ok_block_num;

                    Self::send_block_detection_status(&context, HealthSeverity::Healthy)?;
                    Self::send_time_to_detect_block(&context, &block)?;
                    break;
                }
            }
            Self::send_block_detection_status(&context, HealthSeverity::Critical)?;
        }

        //Fetch data and report metrics
        let timer = Instant::now();

        self.load_data_accesses(&context, block_num).await?;

        Self::send_time_to_fetch_state(&context, timer.elapsed())?;

        Ok(Some(EvmWatcherMessage::Block(block_num)))
    }
}

///Output Message of the [`EvmWatcher`]
#[derive(Debug)]
pub enum EvmWatcherMessage {
    ///Variant denoting a new block was detected
    Block(u64),
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_evm_watcher() {
    use foundry_common::provider::ProviderBuilder;
    use phylax_test_utils::{get_watcher_context, RPC_URL};

    let url = RPC_URL.to_string();
    let chain = Default::default();

    let provider = ProviderBuilder::new(&url).build().unwrap();

    let ctx = get_watcher_context::<EvmWatcher>();

    let db = ctx.state_registry.get::<Backend>().unwrap().clone();

    assert!(EvmWatcher { url, chain, provider, db }
        .process_message(None, ctx)
        .await
        .unwrap()
        .is_some());
}
