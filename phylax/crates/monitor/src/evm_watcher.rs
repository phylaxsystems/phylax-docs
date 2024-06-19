use crate::error::EvmWatcherError;
use color_eyre::Result;
use ethers_core::types::{Block, Chain, H256};
use ethers_providers::{Http, Middleware, Provider, Ws};
use flume::{Receiver, Sender};
use futures_core::Stream;
use phylax_config::EvmWatcherConfig;
use phylax_tracing::tracing::{info, warn};
use std::{
    fmt,
    fmt::{Display, Formatter},
};
use url::Url;

pub struct EvmWatcher {
    pub chain: Chain,
    pub endpoint: Url,
    pub connected_provider: Option<Providers>,
    pub res_rx: Receiver<H256>,
    pub res_tx: Sender<H256>,
}

#[derive(Clone, Debug)]
pub enum Providers {
    Http(Provider<Http>),
    Ws(Provider<Ws>),
}

impl Providers {
    pub async fn block_stream<'a>(
        &'a self,
    ) -> Result<Box<dyn Stream<Item = H256> + 'a + Unpin + Send>> {
        match self {
            Providers::Http(provider) => {
                let stream = provider.watch_blocks().await?;
                let chain_id = provider.get_chainid().await?;
                let chain = Chain::try_from(chain_id)?;
                info!(chain = %chain, "Watcher subscribed to block stream");
                Ok(Box::new(stream))
            }
            Providers::Ws(provider) => {
                let stream = provider.watch_blocks().await?;
                let chain_id = provider.get_chainid().await?;
                let chain = Chain::try_from(chain_id)?;
                info!(
                    chain = %chain,
                    "Watcher subscribed to block stream"
                );
                Ok(Box::new(stream))
            }
        }
    }

    pub async fn get_block_from_hash(&self, hash: H256) -> Result<Block<H256>> {
        let block = match self {
            Providers::Http(provider) => provider.get_block(hash).await?,
            Providers::Ws(provider) => provider.get_block(hash).await?,
        }
        .ok_or(EvmWatcherError::UnknownHash)?;
        Ok(block)
    }
}

impl EvmWatcher {
    pub fn new(chain: Chain, endpoint: Url) -> Self {
        let (res_tx, res_rx) = flume::bounded(50);
        EvmWatcher { chain, endpoint, connected_provider: None, res_tx, res_rx }
    }
    pub async fn connect(&mut self) -> Result<()> {
        self.connected_provider = Some(match self.endpoint.scheme() {
            "ws" | "wss" => {
                Providers::Ws(Provider::<Ws>::connect(self.endpoint.to_string()).await?)
            }
            "http" | "https" => {
                warn!("Watcher is using an HTTP endpoint. It's less efficient than Websockets.");
                Providers::Http(Provider::<Http>::try_from(self.endpoint.to_string())?)
            }
            _ => return Err(EvmWatcherError::UnknownProvider.into()),
        });
        let provider = self.connected_provider.as_ref().unwrap();
        // this is horrible and I hate myself
        // Make sure that the provider is authenticated, else it falls "silently"
        let block = match provider {
            Providers::Http(provider) => provider.get_block_number().await,
            Providers::Ws(provider) => provider.get_block_number().await,
        }?;
        info!(
            chain = %self.chain,
            endpoint=%self.endpoint,
            latest_block = %block,
            "Watcher connected to an RPC endpoint"
        );
        Ok(())
    }
}

impl Display for EvmWatcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "EvmWatcher [ chain: {} ]", self.chain)
    }
}

impl TryFrom<EvmWatcherConfig> for EvmWatcher {
    type Error = EvmWatcherError;
    fn try_from(c: EvmWatcherConfig) -> Result<Self, EvmWatcherError> {
        Ok(Self::new(c.chain, Url::parse(&c.eth_rpc.into_inner())?))
    }
}

#[cfg(test)]
mod tests {
    use phylax_config::SensitiveValue;

    use super::*;
    use std::convert::TryInto;

    #[tokio::test]
    async fn test_try_from_evm_watcher_config() {
        let config = EvmWatcherConfig {
            chain: Chain::Mainnet,
            eth_rpc: SensitiveValue::new("https://rpc.mevblocker.io".to_string()),
            // Add any other fields as necessary
        };

        let watcher: Result<EvmWatcher, _> = config.try_into();
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        assert_eq!(watcher.chain, Chain::Mainnet);
        assert_eq!(watcher.endpoint.as_str(), "https://rpc.mevblocker.io/");
        assert!(watcher.connected_provider.is_none());
    }

    #[tokio::test]
    async fn test_try_from_invalid_evm_watcher_config() {
        let config = EvmWatcherConfig {
            chain: Chain::Mainnet,
            eth_rpc: SensitiveValue::new("invalid_url".to_string()),
        };

        let watcher: Result<EvmWatcher, _> = config.try_into();
        assert!(watcher.is_err());

        match watcher {
            Err(EvmWatcherError::InvalidRpc(_)) => {} // Assert that the error is a UrlParseError
            _ => panic!("Unexpected error type"),
        }
    }

    #[tokio::test]
    async fn test_connect_http() {
        let chain = Chain::Mainnet;
        let endpoint = Url::parse("https://rpc.mevblocker.io").unwrap();
        let mut watcher = EvmWatcher::new(chain, endpoint.clone());

        assert!(watcher.connect().await.is_ok());
        assert!(matches!(watcher.connected_provider, Some(Providers::Http(_))));
    }

    #[tokio::test]
    async fn test_connect_ws() {
        let chain = Chain::Mainnet;
        let endpoint = Url::parse("wss://ethereum.publicnode.com").unwrap();
        let mut watcher = EvmWatcher::new(chain, endpoint.clone());

        assert!(watcher.connect().await.is_ok());
        assert!(matches!(watcher.connected_provider, Some(Providers::Ws(_))));
    }

    #[tokio::test]
    async fn test_connect_invalid() {
        let chain = Chain::Mainnet;
        let endpoint = Url::parse("ftp://localhost:8545").unwrap();
        let mut watcher = EvmWatcher::new(chain, endpoint.clone());

        assert!(watcher.connect().await.is_err());
    }
    #[tokio::test]
    async fn test_get_block_from_hash() {
        let chain = Chain::Mainnet;
        let endpoint = Url::parse("https://rpc.mevblocker.io").unwrap();
        let mut watcher = EvmWatcher::new(chain, endpoint.clone());

        // Connect to the provider
        assert!(watcher.connect().await.is_ok());

        // Use a known block hash for testing
        let block_hash =
            "0x1004255c7b6e3d07e5292f6da91bfc625e62e6571fb7f8651301cbd5958652de".parse().unwrap();

        // Get the block from the hash
        let block_result =
            watcher.connected_provider.as_ref().unwrap().get_block_from_hash(block_hash).await;

        // Check that the block was retrieved successfully
        assert!(block_result.is_ok());

        // Check that the block hash matches the expected hash
        let block: Block<H256> = block_result.unwrap();
        assert_eq!(block.hash.unwrap(), block_hash);
        assert_eq!(block.number.unwrap(), 18757631.into());
        assert_eq!(
            block.author.unwrap(),
            "0x7CDA323561639A3CBb21fFbE6f89B99c48b06d91".parse().unwrap(),
        );
    }
}
