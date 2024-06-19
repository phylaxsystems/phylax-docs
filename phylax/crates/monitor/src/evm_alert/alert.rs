use crate::error::EvmAlertError;
use color_eyre::Result;
use ethers_core::types::Chain;
use foundry_cli::opts::{CoreBuildArgs, ProjectPathsArgs};
use phylax_common::forge::{build::BuildArgs, test::TestArgs};
use phylax_config::EvmAlertConfig;
use phylax_tracing::tracing::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt,
    fmt::{Display, Formatter},
    path::PathBuf,
};

use phylax_common::forge::test::TestOutcome;

/// State: Alert is firing
#[derive(Clone)]
pub struct Firing {
    pub block: u64,
    pub failed_tests: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlertState {
    Uninitialized,
    Firing,
    Off,
}

impl Display for AlertState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AlertState::Uninitialized => write!(f, "uninitialized"),
            AlertState::Firing => write!(f, "firing"),
            AlertState::Off => write!(f, "off"),
        }
    }
}

/// The Alert of a particular AlertTask. It is derived from an AlertConfig, which acts as the
/// blueprint for the Alert.
#[derive(Clone, Debug)]
pub struct EvmAlert {
    pub state: AlertState,
    pub foundry_config_root: PathBuf,
    pub foundry_profile: String,
    pub internal_tx: flume::Sender<EvmAlertResult>,
    pub internal_rx: flume::Receiver<EvmAlertResult>,
}

/// The result of an EVM alert.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvmAlertResult {
    /// The state of the alert.
    pub alert_state: AlertState,
    /// The tests that failed.
    pub failed_tests: HashMap<String, String>,
    /// Whether the setup failed.
    pub failed_setup: bool,
    /// The duration of the alert execution.
    pub duration: std::time::Duration,
    /// All values exposed by the alert.
    pub all_exposed_values: Vec<ExposedValue>,
}

/// A value exposed by an EVM alert.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExposedValue {
    /// The data of the exposed value.
    pub data: String,
    /// The key of the exposed value.
    pub key: String,
    /// The block number of the exposed value.
    pub block_number: u64,
    /// The chain of the exposed value.
    pub chain: Chain,
    /// The context of the exposed value.
    pub context: String,
}

impl ExposedValue {
    /// Checks if the exposed value is a system message.
    pub fn is_system_message(&self, message: &str) -> bool {
        self.key.strip_prefix("phylax_").map(|striped| striped == message).is_some_and(|x| x)
    }
}

impl EvmAlertResult {
    /// Checks if the alert is firing.
    pub fn alert_firing(&self) -> bool {
        match self.alert_state {
            AlertState::Firing => true,
            AlertState::Off => false,
            AlertState::Uninitialized => false,
        }
    }
    /// Creates a pretty string with all the failed tests and their reasons.
    pub fn failed_tests_pretty(&self) -> String {
        self.failed_tests
            .iter()
            .map(|(test, reason)| format!("{}: {}", test, reason))
            .collect::<Vec<String>>()
            .join(" || ")
    }
}

impl EvmAlert {
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

    /// Constructs the test arguments for the EVM alert.
    fn construct_test_args(&self) -> TestArgs {
        TestArgs {
            opts: CoreBuildArgs {
                silent: true,
                project_paths: self.contract_project_paths(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Builds the alerts for the EVM alert.
    pub fn build_alerts(&self) -> Result<(), EvmAlertError> {
        let build_args = self.construct_build_args();
        debug!(args=?build_args, "Running forge build with args");
        let res = build_args.run().map_err(|e| EvmAlertError::BuildAlert { source: e })?;
        if res.has_compiler_errors() {
            let errors = res.output().errors;
            let messages = errors.iter().map(|e| e.message.clone()).collect::<Vec<String>>();
            return Err(EvmAlertError::CompileAlert { errors: messages });
        }
        Ok(())
    }

    /// Runs the tests for the EVM alert.
    pub async fn run_tests(&self, context: HashMap<String, Value>) -> Result<EvmAlertResult> {
        let test_args: TestArgs = self.construct_test_args();
        debug!(target="phylax-monitor::evm_alert", args=?test_args, ?context,  "Running forge test with args");
        let test_outcome = test_args.execute_tests(context).await?;
        debug!(target = "phylax-monitor::evm_alert", ?test_outcome, "Test Outcome");
        let res = test_outcome.into();
        Ok(res)
    }
}

impl Display for EvmAlert {
    /// Formats the EVM alert for display.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Alert [ State: {} ]", self.state)
    }
}

impl From<TestOutcome> for EvmAlertResult {
    /// Converts a test outcome into an EVM alert result.
    fn from(outcome: TestOutcome) -> Self {
        let failures = outcome
            .failures()
            .map(|(name, result)| {
                (name.to_string(), result.reason.clone().unwrap_or("No revert info".to_owned()))
            })
            .collect::<HashMap<String, String>>();
        let all_exposed_values = outcome
            .results
            .iter()
            .flat_map(|(contract, suite_result)| {
                suite_result
                    .exported_data()
                    .into_iter()
                    .map(|exported_data| ExposedValue {
                        data: exported_data.value,
                        key: exported_data.name,
                        block_number: exported_data.block,
                        chain: exported_data.chain,
                        context: contract.to_owned(),
                    })
                    .collect::<Vec<ExposedValue>>()
            })
            .collect::<Vec<ExposedValue>>();
        let failed_setup = failures.get("setUp()").or_else(|| failures.get("setup()")).is_some();
        let state =
            if failures.is_empty() || failed_setup { AlertState::Off } else { AlertState::Firing };
        Self {
            alert_state: state,
            failed_tests: failures,
            failed_setup,
            duration: outcome.duration(),
            all_exposed_values,
        }
    }
}

impl From<EvmAlertConfig> for EvmAlert {
    fn from(config: EvmAlertConfig) -> Self {
        let (internal_tx, internal_rx) = flume::bounded(10);
        Self {
            state: AlertState::Off,
            foundry_config_root: config.foundry_project_root_path,
            foundry_profile: config.foundry_profile,
            internal_rx,
            internal_tx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            foundry_profile: "simple".to_string(),
            internal_tx,
            internal_rx: internal_rx.clone(),
        }
    }

    #[tokio::test]
    async fn test_build_alerts() {
        let evm_alert = create_test_alert();
        let result = evm_alert.build_alerts();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_alerts() {
        let evm_alert = create_test_alert();
        let context = HashMap::new();
        let result = evm_alert.run_tests(context).await;
        assert!(result.is_ok());
        // rest are tested in the coreNG crate
    }
}
