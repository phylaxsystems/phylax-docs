use async_trait::async_trait;
use color_eyre::{Report, Result};
use flume::r#async::RecvStream;

use phylax_respond::{EvmAction, EvmActionRes};

use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use crate::{
    activities::{Activity, ActivityContext, FgWorkType},
    events::{Event, EventBuilder, EventType},
    tasks::task::WorkContext,
};

/// Enum representing the commands that the EvmAction activity can execute.
#[derive(Debug, PartialEq)]
pub enum EvmActionCmd {
    /// Command to run the action.
    RunAction,
}

/// Implementation of Display trait for EvmActionCmd.
impl Display for EvmActionCmd {
    /// Formats the EvmActionCmd variants into a string.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EvmActionCmd::RunAction => write!(f, "RunAction"),
        }
    }
}

/// Implementation of Activity trait for EvmAction.
#[async_trait]
impl Activity for EvmAction {
    type Command = EvmActionCmd;
    type InternalStream = RecvStream<'static, Self::InternalStreamItem>;
    type InternalStreamItem = EvmActionRes;

    /// Bootstraps the EvmAction activity. During bootstrap it runs a simulation of the action
    /// against the on-chain state. If it reverts during the simulation, the bootstrap will
    /// fail.
    async fn bootstrap(&mut self, ctx: Arc<ActivityContext>) -> Result<()> {
        let context_map = ctx.get_map();
        self.simulate_execution(context_map).await.map_err(Report::from)?;
        Ok(())
    }

    /// Decides the next command to be executed. Currently there is only one command
    /// and that's to execute the forge script.
    fn decide(
        &self,
        _events: Vec<&Event>,
        _context: &WorkContext,
    ) -> std::option::Option<Vec<EvmActionCmd>> {
        Some(vec![EvmActionCmd::RunAction])
    }

    /// Executes the commands. Execute the forge script and broadcast the transactions.
    async fn do_work(&mut self, cmds: Vec<EvmActionCmd>, context: WorkContext) -> Result<()> {
        let context_map = context.into_map();
        for cmd in cmds {
            match cmd {
                EvmActionCmd::RunAction => {
                    let res = self.execute_onchain(context_map.clone()).await?;
                    self.response_sender.send(res)?;
                }
            }
        }
        Ok(())
    }

    /// Returns the work type of the activity.
    fn work_type(&self) -> FgWorkType {
        FgWorkType::SpawnBlocking
    }

    /// Returns the internal stream of the activity.
    async fn internal_stream(&self) -> Option<Self::InternalStream> {
        Some(self.response_stream.clone())
    }

    /// Handles work from the internal stream.
    async fn internal_stream_work(
        &mut self,
        item: Self::InternalStreamItem,
        _ctx: Arc<ActivityContext>,
    ) -> Option<Event> {
        let event = item.into();
        Some(event)
    }
}

/// Implementation of From trait for converting EvmActionRes to Event.
impl From<EvmActionRes> for Event {
    /// Converts EvmActionRes to Event.
    fn from(res: EvmActionRes) -> Self {
        EventBuilder::new()
            .with_type(EventType::ActionExecution)
            .with_body(serde_json::to_value(res).unwrap())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::mock_context::mock_activity_context;
    use anvil::{spawn, NodeConfig};
    use ethers_core::types::{Chain, H256};
    use phylax_respond::EvmActionWallet;
    use std::{path::PathBuf, str::FromStr};
    use tokio_stream::StreamExt;

    fn create_evm_action() -> EvmAction {
        let profile = "test_profile".to_string();
        let foundry_profile_root = PathBuf::from("/test/path");
        let contract_name = "test_contract".to_string();
        let function_signature = "test_signature".to_string();
        let function_args = vec!["arg1".to_string(), "arg2".to_string()];
        let wallet = EvmActionWallet::PrivateKey("test_private_key".to_owned());

        EvmAction::new(
            profile,
            foundry_profile_root,
            contract_name,
            function_signature,
            function_args,
            wallet,
        )
    }
    #[test]
    fn test_work_type() {
        let evm_action = create_evm_action();
        assert_eq!(evm_action.work_type(), FgWorkType::SpawnBlocking);
    }

    #[test]
    fn test_decide() {
        let evm_action = create_evm_action();
        let events = vec![]; // replace with actual events if needed
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        assert_eq!(evm_action.decide(events, &context), Some(vec![EvmActionCmd::RunAction]));

        let event = EventBuilder::new().build();
        let event2 = EventBuilder::new().with_origin(43).build();
        assert_eq!(
            evm_action.decide(vec![&event, &event2], &context),
            Some(vec![EvmActionCmd::RunAction])
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_anvil_do_work() {
        let (api, handle) = spawn(NodeConfig::test()).await;
        api.anvil_set_interval_mining(1).unwrap();
        // account #1
        let accounts_priv =
            ["ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()];
        let endpoint = handle.http_endpoint();
        std::env::set_var("ANVIL", &endpoint);

        let workspace_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let mut path = std::path::Path::new(&workspace_root)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("testdata/actions");

        let profile = "simple".to_owned();
        let contract_name = "SimpleAction".to_owned();
        let function_signature = "run()".to_owned();
        let function_args = Vec::new();
        let wallet = EvmActionWallet::PrivateKey(accounts_priv[0].clone());
        let mut evm_action = EvmAction::new(
            profile.clone(),
            path.clone(),
            contract_name,
            function_signature,
            function_args,
            wallet,
        );

        let project = ethers_solc::project_util::TempProject::dapptools().unwrap();
        path.push("simple/ExampleContract.sol");
        project.copy_source(path).unwrap();
        let mut output = project.compile().unwrap();
        let contract = output
            .remove_first("ExampleContract")
            .unwrap()
            .deployed_bytecode
            .unwrap()
            .bytecode
            .unwrap();
        let address = "0x000000000000000000000000000000000000bEEF".parse().unwrap();
        api.anvil_set_code(address, contract.object.as_bytes().unwrap().to_owned()).await.unwrap();

        let mut stream = evm_action.internal_stream().await.unwrap();

        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let cmds = vec![EvmActionCmd::RunAction];
        evm_action.do_work(cmds, context).await.unwrap();

        let res: EvmActionRes = stream.next().await.unwrap();
        assert_eq!(res.receipts.get(&Chain::AnvilHardhat).unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_anvil_do_work_revert() {
        let (api, handle) = spawn(NodeConfig::test()).await;
        api.anvil_set_interval_mining(1).unwrap();
        let accounts_priv =
            ["ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()];
        let endpoint = handle.http_endpoint();
        std::env::set_var("ANVIL", &endpoint);

        let workspace_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let mut path = std::path::Path::new(&workspace_root)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("testdata/actions");

        let profile = "simple".to_owned();
        let contract_name = "SimpleActionRevert".to_owned();
        let function_signature = "run()".to_owned();
        let function_args = Vec::new();
        let wallet = EvmActionWallet::PrivateKey(accounts_priv[0].clone());
        let mut evm_action = EvmAction::new(
            profile.clone(),
            path.clone(),
            contract_name,
            function_signature,
            function_args,
            wallet,
        );

        let project = ethers_solc::project_util::TempProject::dapptools().unwrap();
        path.push("simple/ExampleContract.sol");
        project.copy_source(path).unwrap();
        let mut output = project.compile().unwrap();
        let contract = output
            .remove_first("ExampleContract")
            .unwrap()
            .deployed_bytecode
            .unwrap()
            .bytecode
            .unwrap();
        let address = "0x000000000000000000000000000000000000bEEF".parse().unwrap();
        api.anvil_set_code(address, contract.object.as_bytes().unwrap().to_owned()).await.unwrap();

        let stream = evm_action.internal_stream().await.unwrap();

        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let cmds = vec![EvmActionCmd::RunAction];
        let res = evm_action.do_work(cmds, context).await;
        assert!(res.as_ref().is_err());
        let error = res.unwrap_err();
        let errors: Vec<String> = error.chain().map(|e| e.to_string()).collect();
        assert_eq!(errors[0], "Failed to execute the script");
        assert_eq!(errors[1], "Script reverted without a revert string");
        assert!(stream.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_anvil_bootstrap() {
        let (api, handle) = spawn(NodeConfig::test()).await;
        api.anvil_set_interval_mining(1).unwrap();
        let accounts_priv =
            ["ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()];
        let endpoint = handle.http_endpoint();
        std::env::set_var("ANVIL", &endpoint);

        let workspace_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let mut path = std::path::Path::new(&workspace_root)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("testdata/actions");

        let profile = "simple".to_owned();
        let contract_name = "SimpleAction".to_owned();
        let function_signature = "run()".to_owned();
        let function_args = Vec::new();
        let wallet = EvmActionWallet::PrivateKey(accounts_priv[0].clone());
        let mut evm_action = EvmAction::new(
            profile.clone(),
            path.clone(),
            contract_name,
            function_signature,
            function_args,
            wallet,
        );

        let project = ethers_solc::project_util::TempProject::dapptools().unwrap();
        path.push("simple/ExampleContract.sol");
        project.copy_source(path).unwrap();
        let mut output = project.compile().unwrap();
        let contract = output
            .remove_first("ExampleContract")
            .unwrap()
            .deployed_bytecode
            .unwrap()
            .bytecode
            .unwrap();
        let address = "0x000000000000000000000000000000000000bEEF".parse().unwrap();
        api.anvil_set_code(address, contract.object.as_bytes().unwrap().to_owned()).await.unwrap();

        let activity_context = mock_activity_context(None);
        let stream = evm_action.internal_stream().await.unwrap();

        // Bootstrap should simulate the correct execution but not realise any change on the chain
        evm_action.bootstrap(activity_context.clone()).await.unwrap();
        let data = api.storage_at(address, "0".parse().unwrap(), None).await.unwrap();
        assert_eq!(data, H256::zero());
        assert!(stream.is_empty());

        // Do work must realise a change in the chain
        let cmds = vec![EvmActionCmd::RunAction];
        let context = WorkContext::new(activity_context, EventBuilder::new().build());
        evm_action.do_work(cmds, context).await.unwrap();
        let data = api.storage_at(address, "0".parse().unwrap(), None).await.unwrap();
        assert_eq!(
            data,
            H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap()
        );
        assert!(!stream.is_empty());
    }
}
