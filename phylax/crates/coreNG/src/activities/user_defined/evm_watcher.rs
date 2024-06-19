use crate::{
    activities::{Activity, ActivityContext, FgWorkType},
    events::{Event, EventBuilder, EventType},
};
use async_trait::async_trait;
use ethers_core::types::{Address, Block, H256, U64};
use eyre::Result;
use flume::r#async::RecvStream;
use futures_util::StreamExt;
use phylax_monitor::EvmWatcher;
use phylax_tracing::tracing::{info_span, Instrument};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::SystemTime};
use tokio::task::JoinHandle;

#[async_trait]
impl Activity for EvmWatcher {
    /// This type is not used by EvmWatcher
    type Command = u32;
    /// The stream of blocks
    type InternalStream = RecvStream<'static, Self::InternalStreamItem>;
    /// block hashes
    type InternalStreamItem = H256;

    /// Connect to the RPC endpoint and make a test request
    /// Fail the bootstrap if the request fails.
    async fn bootstrap(&mut self, _ctx: Arc<ActivityContext>) -> Result<()> {
        self.connect().await?;
        Ok(())
    }

    fn work_type(&self) -> FgWorkType {
        FgWorkType::SpawnNonBlocking
    }

    async fn internal_stream(&self) -> Option<Self::InternalStream> {
        let stream = self.res_rx.clone().into_stream();
        Some(stream)
    }

    /// Spawns the background work that watches for new blocks in the blockchain
    fn spawn_bg_work(&mut self) -> Option<JoinHandle<Result<()>>> {
        let span = info_span!("background work");
        // Get the provider
        let provider = self.connected_provider.clone().unwrap();
        // Get a clone of the internal stream sender
        let sender = self.res_tx.clone();
        let handle = tokio::task::spawn(
            async move {
                // The block stream
                let mut stream = provider.block_stream().await?;
                loop {
                    // get the hash of the latest block
                    let maybe_hash = stream.next().await;
                    if let Some(hash) = maybe_hash {
                        sender.send(hash)?
                    }
                }
            }
            .instrument(span),
        );
        Some(handle)
    }
    /// Convert a block hash ([`Activity::InternalStreamItem`]) into a phylax event
    /// It also makes an RPC request to get more inforamtion about the block, so that
    /// the internal event is enriched with added metadata.
    /// This can change in the feature as it adds some considerable lag in the process
    // TODO(odysseas): revisit this
    async fn internal_stream_work(
        &mut self,
        item: Self::InternalStreamItem,
        ctx: Arc<ActivityContext>,
    ) -> Option<Event> {
        let res: EvmWatcherRes = self
            .connected_provider
            .as_ref()
            .unwrap()
            .get_block_from_hash(item)
            .await
            .unwrap()
            .into();
        ctx.metrics.set_watcher_network_lag(&ctx.task_name, res.latency);
        ctx.metrics.set_watcher_block_number(&ctx.task_name, res.block_number);
        let event = res.into();
        Some(event)
    }
}

/// The information that is sent in the event body when a new block is found by the watcher
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EvmWatcherRes {
    /// The block number
    pub block_number: u64,
    /// The block hash
    pub block_hash: H256,
    /// The block UNIX timestamp
    pub block_timestamp: u64,
    /// How many seconds it took for the watcher to detect the block from the time it was built
    pub latency: i64,
    /// The EVM address of the block proposer
    pub author: Address,
}

impl From<Block<H256>> for EvmWatcherRes {
    fn from(block: Block<H256>) -> Self {
        Self {
            block_number: block.number.unwrap_or_else(U64::zero).as_u64(),
            block_hash: block.hash.unwrap_or_else(H256::zero),
            block_timestamp: block.timestamp.as_u64(),
            latency: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
                as i64 -
                block.timestamp.as_u64() as i64,
            author: block.author.unwrap_or_else(|| Address::from(H256::zero())),
        }
    }
}

impl From<EvmWatcherRes> for Event {
    fn from(val: EvmWatcherRes) -> Self {
        EventBuilder::new()
            .with_type(EventType::NewBlock)
            .with_body(serde_json::to_value(val).unwrap())
            .build()
    }
}
#[cfg(test)]
mod tests {
    use crate::mocks::mock_context::mock_activity_context;

    use super::*;
    use anvil::{spawn, NodeConfig};
    use ethers_core::types::{Chain, U256};
    use once_cell::sync::OnceCell;

    static BLOCK_TIMESTAMP: OnceCell<u64> = OnceCell::new();
    static BLOCK_NUMBER: OnceCell<u64> = OnceCell::new();
    static BLOCK_AUTHOR: OnceCell<Address> = OnceCell::new();

    async fn create_watcher(endpoint_type: &str) -> EvmWatcher {
        let (api, handle) = spawn(NodeConfig::test()).await;
        BLOCK_NUMBER.set(123).unwrap();
        api.anvil_set_block(U256::from(123)).unwrap();
        BLOCK_TIMESTAMP
            .set(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() - 12)
            .unwrap();
        api.evm_set_time(BLOCK_TIMESTAMP.get().unwrap().to_owned()).unwrap();
        BLOCK_AUTHOR.set(api.author().unwrap()).unwrap();
        api.anvil_set_interval_mining(1).unwrap();
        let (res_tx, res_rx) = flume::bounded(10);
        let endpoint = match endpoint_type {
            "ws" => handle.ws_endpoint(),
            "http" => handle.http_endpoint(),
            _ => panic!("unknown endpoint type"),
        };
        let endpoint = endpoint.parse().unwrap();
        EvmWatcher { chain: Chain::Sepolia, endpoint, connected_provider: None, res_rx, res_tx }
    }
    #[tokio::test]
    async fn test_anvil_ws_bootstrap_watcher() {
        let mut watcher = create_watcher("ws").await;
        let context = mock_activity_context(None);
        assert!(watcher.bootstrap(context).await.is_ok());
        assert!(watcher.connected_provider.is_some());
    }
    #[tokio::test]
    async fn test_anvil_http_bootstrap_watcher() {
        let mut watcher = create_watcher("http").await;
        let context = mock_activity_context(None);
        assert!(watcher.bootstrap(context).await.is_ok());
        assert!(watcher.connected_provider.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_anvil_ws_spawn_bg_work() {
        let mut watcher = create_watcher("ws").await;
        let context = mock_activity_context(Some("test_watcher".to_owned()));
        watcher.bootstrap(context.clone()).await.unwrap();
        let _handle = watcher.spawn_bg_work().unwrap();
        let mut stream = watcher.internal_stream().await.unwrap();
        let block: H256 = stream.next().await.unwrap();
        assert_ne!(block, H256::zero());
        let event = watcher.internal_stream_work(block, context.clone()).await.unwrap();
        let body: EvmWatcherRes = serde_json::from_value(event.get_body()).unwrap();
        assert_eq!(body.block_hash, block);
        assert_eq!(body.block_number, *(BLOCK_NUMBER.get().unwrap()) + 1);
        assert_eq!(body.block_timestamp, *(BLOCK_TIMESTAMP.get().unwrap()) + 1);
        assert_eq!(&body.author, BLOCK_AUTHOR.get().unwrap());
        // the watcher seems to have a minimum latency of 6 seconds?
        // TODO(odysseas): Remove the extra request needed to fetch thre block from the hash
        // to reduce the latency
        assert_ne!(body.latency, 0);
        assert_eq!(
            context.metrics.get_watcher_block_number("test_watcher"),
            BLOCK_NUMBER.get().unwrap().to_owned() as i64 + 1,
        );
        assert_eq!(context.metrics.get_watcher_network_lag("test_watcher"), body.latency);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_anvil_http_spawn_bg_work() {
        let mut watcher = create_watcher("http").await;
        let context = mock_activity_context(Some("test_watcher".to_owned()));
        watcher.bootstrap(context.clone()).await.unwrap();
        let _handle = watcher.spawn_bg_work().unwrap();
        let mut stream = watcher.internal_stream().await.unwrap();
        let block: H256 = stream.next().await.unwrap();
        assert_ne!(block, H256::zero());
        let event = watcher.internal_stream_work(block, context.clone()).await.unwrap();
        let body: EvmWatcherRes = serde_json::from_value(event.get_body()).unwrap();
        assert_eq!(body.block_hash, block);
        assert_eq!(body.block_number, *(BLOCK_NUMBER.get().unwrap()) + 1);
        assert_eq!(body.block_timestamp, *(BLOCK_TIMESTAMP.get().unwrap()) + 1);
        assert_eq!(&body.author, BLOCK_AUTHOR.get().unwrap());
        // the watcher seems to have a minimum latency of 6 seconds?
        // TODO(odysseas): Remove the extra request needed to fetch thre block from the hash
        // to reduce the latency
        assert_ne!(body.latency, 0);
        assert_eq!(
            context.metrics.get_watcher_block_number("test_watcher"),
            BLOCK_NUMBER.get().unwrap().to_owned() as i64 + 1,
        );
        assert_eq!(context.metrics.get_watcher_network_lag("test_watcher"), body.latency);
    }
}
