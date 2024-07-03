use std::{path::PathBuf, str::FromStr};

use phylax_config::{sensitive::SensitiveValue, substitute_env::ValOrEnvVar};
use serde::{Deserialize, Serialize};

/// Configuration for an action that executes a forge script.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EvmActionConfig {
    /// The name of the script's contract.
    pub contract_name: String,
    /// The signature of the entrypoint function. If `None`, it's assumed to be `run()`.
    #[serde(default = "EvmActionConfig::default_signature")]
    pub function_signature: String,
    /// The arguments of the entrypoint function, supporting ENV variables in the format
    /// `$ENV_VARIABLE`.
    #[serde(default = "EvmActionConfig::default_arguments")]
    pub function_arguments: Vec<ValOrEnvVar<String>>,
    /// Configuration for the wallet and address used to send the action's transactions, supporting
    /// ENV variables.
    pub wallet: ValOrEnvVar<SensitiveValue<WalletKind>>,
    /// The path to the Solidity file containing the contract.
    pub path: PathBuf,
}

impl EvmActionConfig {
    /// Default signature for the function if none is provided.
    fn default_signature() -> String {
        "run()".into()
    }

    /// Default arguments for the function if none are provided.
    fn default_arguments() -> Vec<ValOrEnvVar<String>> {
        vec![]
    }
}

/// Configuration for the transaction sender action. All transactions are sent serially from the
/// same wallet. For transactions from different users, define a new action triggered from the same
/// alert.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EvmTxConfig {
    /// List of transaction descriptions.
    pub transactions: Vec<String>,
    /// Wallet configuration supporting ENV variables.
    pub wallet: ValOrEnvVar<SensitiveValue<WalletKind>>,
    /// RPC endpoints for the transactions, supporting sensitive ENV variables.
    pub rpc_endpoints: Vec<ValOrEnvVar<SensitiveValue<String>>>,
    /// Priority of the gas to be used in transactions.
    pub gas_priority: usize,
}

/// Represents the type of wallet used by a `ForgeScriptAction`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(untagged)]
pub enum WalletKind {
    /// Wallet using a raw private key.
    PrivateKey(String),
    /// Wallet using AWS KMS for signing, expecting certain AWS environment variables.
    #[serde(rename = "aws")]
    #[default]
    Aws,
}

impl FromStr for WalletKind {
    type Err = std::string::ParseError;

    /// Parses a string to return a WalletKind.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Aws" | "aws" => Ok(WalletKind::Aws),
            _ => Ok(WalletKind::PrivateKey(s.to_owned())),
        }
    }
}
