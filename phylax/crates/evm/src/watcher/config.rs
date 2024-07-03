use alloy_chains::NamedChain;
use phylax_config::{sensitive::SensitiveValue, substitute_env::ValOrEnvVar};
use serde::{Deserialize, Serialize};

/// Configuration for an EVM watcher.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvmWatcherConfig {
    /// The blockchain chain identifier.
    chain: NamedChain,
    /// The specific kind of EVM watcher configuration, flattened in serialization.
    #[serde(flatten)]
    kind: EvmWatcherKind,
}

/// Represents different kinds of EVM watcher configurations.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum EvmWatcherKind {
    /// Configuration for a JSON RPC based EVM watcher.
    JsonRpc(EvmJsonRpcWatcherConfig),
}

/// Configuration for an EVM watcher that uses JSON RPC.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvmJsonRpcWatcherConfig {
    /// A list of endpoints for the JSON RPC, potentially sensitive, supporting environment
    /// variable substitution.
    endpoints: Vec<ValOrEnvVar<SensitiveValue<String>>>,
    /// The strategy used by the JSON RPC watcher.
    strategy: EvmJsonRpcStrategy,
}

/// Different strategies for JSON RPC EVM watching.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub enum EvmJsonRpcStrategy {
    /// Automatically determine the best strategy based on network conditions and capabilities.
    #[default]
    Auto,
    /// A naive strategy that simply polls at regular intervals.
    Naive,
    /// A strategy that listens for state differences to detect changes.
    StateDiffs,
}
