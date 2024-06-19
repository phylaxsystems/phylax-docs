use crate::{substitute_env::deserialize_with_env, SensitiveValue};
use ethers_core::types::Chain;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct WatcherConfig {
    #[serde(deserialize_with = "deserialize_with_env")]
    pub name: String,
    #[serde(flatten)]
    pub kind: WatcherKind,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum WatcherKind {
    Evm(EvmWatcherConfig),
}
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvmWatcherConfig {
    /// The chain that the watchers observes
    #[serde(deserialize_with = "deserialize_with_env")]
    pub chain: Chain,
    /// The rpc endpoint that will subscribe to.
    /// Supports ENV variables substitutions as such: $ENV_VARIABLE
    // #[serde(deserialize_with = "de_ws_or_http")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub eth_rpc: SensitiveValue<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Test substitution of environment variables in EvmWatcherConfig
    #[test]
    fn test_sub_env_vars() {
        let json = json!(
            {
            "chain": Chain::Mainnet,
            "ethRpc": "$TEST_RPC",
            }
        );
        std::env::set_var("TEST_RPC", "http://localhost:8545");
        let config: EvmWatcherConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.eth_rpc.expose(), "http://localhost:8545");
    }

    /// Test sensitive value exposure
    #[test]
    fn test_sensitive_value() {
        let sensitive_value = SensitiveValue::new("secret".to_string());
        assert_eq!(sensitive_value.expose(), "secret");
        assert_eq!(sensitive_value.to_string(), "<SENSITIVE>".to_owned());
    }
}
