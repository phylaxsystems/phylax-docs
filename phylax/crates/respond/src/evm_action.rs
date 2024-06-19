use crate::error::EvmActionError;
use color_eyre::Result;
use ethers_core::types::Chain;
use flume::{r#async::RecvStream, Sender};
use foundry_cli::opts::{CoreBuildArgs, MultiWallet, ProjectPathsArgs};
use phylax_common::forge::{
    build::BuildArgs,
    script::{ScriptArgs, ScriptArtifact},
};
use phylax_config::{EvmActionConfig, WalletKind};
use phylax_tracing::tracing::debug;
use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::UNIX_EPOCH,
};

pub struct EvmAction {
    /// Foundry profile
    pub foundry_profile: String,
    /// The directory of the foundry_profile of the project
    pub foundry_config_root: PathBuf,
    /// The contract name of the script
    pub contract_name: String,
    /// The signature of the function that will be invoked
    pub function_signature: String,
    /// The arguments of the function that will be invoked
    pub function_args: Vec<String>,
    /// The wallet that will be used
    pub wallet: EvmActionWallet,
    /// The timestamp of the last time that the script was executed
    pub last_execution: Arc<Mutex<u64>>,
    /// The stream of the results from the script execution
    pub response_stream: RecvStream<'static, EvmActionRes>,
    pub response_sender: Sender<EvmActionRes>,
}

impl EvmAction {
    /// Creates a new [`EvmAction`].
    pub fn new(
        profile: String,
        foundry_profile_root: PathBuf,
        contract_name: String,
        function_signature: String,
        function_args: Vec<String>,
        wallet: EvmActionWallet,
    ) -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            foundry_profile: profile,
            foundry_config_root: foundry_profile_root,
            contract_name,
            function_signature,
            function_args,
            wallet,
            last_execution: Arc::new(Mutex::new(UNIX_EPOCH.elapsed().unwrap().as_secs())),
            response_sender: tx,
            response_stream: rx.into_stream(),
        }
    }
}

impl Default for EvmAction {
    fn default() -> Self {
        Self::new(
            "default".to_string(),
            String::from("").into(),
            String::from(""),
            String::from(""),
            vec![],
            EvmActionWallet::PrivateKey("test key".to_owned()),
        )
    }
}

impl EvmAction {
    const DEFAULT_GAS_MULTIPLIER: u64 = 130;
    /// Constructs the project paths for the EVM alert.
    fn contract_project_paths(&self) -> ProjectPathsArgs {
        ProjectPathsArgs {
            profile: Some(self.foundry_profile.clone()),
            root: Some(self.foundry_config_root.clone()),
            ..Default::default()
        }
    }
    /// Constructs the build arguments for the EVM alert.
    fn construct_build_args(&self) -> BuildArgs {
        BuildArgs {
            args: CoreBuildArgs {
                silent: true,
                project_paths: self.contract_project_paths(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn construct_wallet(&self) -> MultiWallet {
        match &self.wallet {
            EvmActionWallet::PrivateKey(key) => {
                MultiWallet { private_key: Some(key.to_owned()), ..Default::default() }
            }
            EvmActionWallet::Aws => {
                debug!("Evm Action using AWS signers");
                MultiWallet { aws: true, ..Default::default() }
            }
        }
    }

    fn construct_script_args(&self, broadcast: bool) -> ScriptArgs {
        ScriptArgs {
            path: self.contract_name.clone(),
            args: self.function_args.clone(),
            target_contract: Some(self.contract_name.clone()),
            sig: self.function_signature.clone(),
            opts: self.construct_build_args(),
            wallets: self.construct_wallet(),
            broadcast,
            gas_estimate_multiplier: Self::DEFAULT_GAS_MULTIPLIER,
            ..Default::default()
        }
    }
    /// Simulate the execution of the forge script
    pub async fn simulate_execution(
        &self,
        context_map: HashMap<String, serde_json::Value>,
    ) -> Result<EvmActionRes, EvmActionError> {
        let script_args = self.construct_script_args(false);
        let raw_res = self
            .run_forge_script(script_args, context_map)
            .await
            .map_err(|e| EvmActionError::ExecuteScript { source: e })?;
        let res: EvmActionRes = raw_res.into();
        Ok(res)
    }

    /// Execute the forge script and broadcast the transactions
    pub async fn execute_onchain(
        &self,
        context_map: HashMap<String, serde_json::Value>,
    ) -> Result<EvmActionRes, EvmActionError> {
        let script_args = self.construct_script_args(true);
        let raw_res = self
            .run_forge_script(script_args, context_map)
            .await
            .map_err(|e| EvmActionError::ExecuteScript { source: e })?;
        let res: EvmActionRes = raw_res.into();
        Ok(res)
    }

    /// Run the forge script with the given arguments
    async fn run_forge_script(
        &self,
        args: ScriptArgs,
        context_map: HashMap<String, serde_json::Value>,
    ) -> Result<ScriptArtifact> {
        debug!(target = "phylax-respond::evm_action", ?args, "forge script args");
        args.run_script(context_map).await
    }
}

impl TryFrom<EvmActionConfig> for EvmAction {
    type Error = EvmActionError;
    fn try_from(config: EvmActionConfig) -> Result<Self, Self::Error> {
        Ok(Self::new(
            config.profile,
            config.foundry_project_root_path,
            config.contract_name,
            config.function_signature,
            config.function_arguments,
            config.wallet.into_inner().into(),
        ))
    }
}

#[derive(PartialEq, Clone)]
pub enum EvmActionWallet {
    PrivateKey(String),
    Aws,
}

impl From<WalletKind> for EvmActionWallet {
    fn from(wallet: WalletKind) -> Self {
        match wallet {
            WalletKind::PrivateKey(key) => EvmActionWallet::PrivateKey(key),
            WalletKind::Aws => EvmActionWallet::Aws,
        }
    }
}

/// The result of a script execution
#[derive(Serialize, Debug)]
pub struct EvmActionRes {
    /// The receipts of the transactions that were broadcasted succesfuly
    pub receipts: HashMap<Chain, Vec<String>>,
    /// The UNIX timestamp of the execution
    pub timestamp: u64,
}

/// Converts a `ScriptArtifact` into an `EvmActionRes`.
impl From<ScriptArtifact> for EvmActionRes {
    fn from(artifact: ScriptArtifact) -> Self {
        let mut receipts_per_chain: HashMap<Chain, Vec<String>> = HashMap::new();
        let timestamp = UNIX_EPOCH.elapsed().map(|dur| dur.as_secs()).unwrap_or(0);
        for (chain, receipts) in artifact.receipts {
            receipts_per_chain.insert(
                chain,
                receipts.iter().map(|r| format!("{:?}", r.transaction_hash)).collect(),
            );
        }
        Self::new(receipts_per_chain, timestamp)
    }
}
impl EvmActionRes {
    pub fn new(receipts: HashMap<Chain, Vec<String>>, timestamp: u64) -> Self {
        Self { receipts, timestamp }
    }
}

impl Display for EvmAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "EvmAction [ script: {} ]", self.contract_name)
    }
}
