use crate::{
    accesses::AccessDiff, runner::tracing::Span, AssertionOutcome, EvmCompilationArtifacts,
    FilterArgs, PhylaxRunnerError,
};
use foundry_evm::inspectors::CheatsConfig;
use phylax_tracing::tracing::{self, debug, instrument, trace};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use forge::{
    backend::Backend,
    executors::ExecutorBuilder,
    multi_runner::TestContract,
    opts::EvmOpts,
    result::{SuiteResult, TestOutcome},
    ContractRunner, MultiContractRunner, MultiContractRunnerBuilder, TestOptions,
    TestOptionsBuilder,
};
use foundry_evm::backend::Access;

use foundry_cli::{opts::CoreBuildArgs, utils::LoadConfig};
use foundry_common::get_contract_name;
use foundry_compilers::{ArtifactId, ProjectCompileOutput};
use foundry_config::{
    cache::{CachedEndpoints, StorageCachingConfig},
    Config,
};

use std::{
    collections::HashSet,
    fmt::{self, Debug},
    path::PathBuf,
    sync::{
        mpsc::{self, channel},
        Arc, RwLock,
    },
};

/// Phylax Assertion Runner.
/// This runner is used to run a set of assertions.
///
/// # Example
/// ```no_run
/// use phylax_runner::{PhylaxRunner, FilterArgs, CoreBuildArgs, Compile, Backend, PhylaxRunnerError};
///
/// #[tokio::main]
/// async fn main() -> Result<(), PhylaxRunnerError>{
///    
///    let artifacts =  CoreBuildArgs::default().compile().unwrap();
///    let filter = FilterArgs::default();
///    let runner = PhylaxRunner::new(artifacts, filter)?;
///
///    let db = Backend::spawn(None);
///
///    //Execute assertions against shared backend
///    let result = runner.execute_assertions(db);
///
///    Ok(())
/// }
/// ```
pub struct PhylaxRunner {
    /// Test Options
    test_options: TestOptions,
    /// Filter
    filter: FilterArgs,
    /// Runner
    runner: Arc<MultiContractRunner>,
    /// Accesses from previous run
    pub prev_accesses: RwLock<Vec<Access>>,
}

impl Clone for PhylaxRunner {
    fn clone(&self) -> Self {
        PhylaxRunner {
            test_options: self.test_options.clone(),
            filter: self.filter.clone(),
            runner: self.runner.clone(),
            prev_accesses: RwLock::new(vec![]),
        }
    }
}

impl Debug for PhylaxRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PhylaxRunner")
            .field("test_options", &self.test_options)
            .field("filter", &self.filter)
            .field("prev_accesses", &self.prev_accesses)
            .finish()
    }
}

impl PhylaxRunner {
    /// Creates a new [`PhylaxRunner`].
    /// for the given [`EvmCompilationArtifacts`] and [`FilterArgs`].
    /// # Arguments
    /// * `EvmCompilationArtifacts` - The compilation artifacts for the project.
    /// * `FilterArgs` - The filter to use for the assertions.
    /// # Returns
    /// * `Result<PhylaxRunner, PhylaxRunnerError>` - The runner instance.
    /// # Errors
    /// * `PhylaxRunnerError` - If the runner could not be created.
    pub fn new(
        EvmCompilationArtifacts { output, project_root, opts }: EvmCompilationArtifacts,
        filter: FilterArgs,
    ) -> Result<Self, PhylaxRunnerError> {
        let test_options = TestOptionsBuilder::default().build(&output, project_root.as_path())?;

        let config = Self::get_config(&opts);

        let runner = Arc::new(Self::get_multi_runner(config, project_root, output)?);

        Ok(PhylaxRunner { test_options, filter, runner, prev_accesses: RwLock::new(vec![]) })
    }

    /// Executes all the assertions for the project, matching the filter.
    ///
    /// # Arguments
    /// * `db` - The backend to use for the assertions.
    ///
    /// # Returns
    /// * `AssertionOutcome` - The outcome of the assertions.
    #[instrument(skip_all, fields(contract_pattern = ?self.filter.contract_pattern))]
    pub fn execute_assertions(&self, db: Backend) -> (AssertionOutcome, AccessDiff) {
        let (tx, rx) = channel::<(String, (SuiteResult, Vec<Access>))>();

        // Run assertions.
        self.execute_assertion_contracts(db, tx);

        let mut recorded_accesses = vec![];
        let outcome = rx.into_iter().fold(
            TestOutcome::empty(false),
            |mut outcome, (contract_name, (suite_result, accesses))| {
                outcome.results.insert(contract_name, suite_result);
                recorded_accesses.extend(accesses);
                outcome
            },
        );

        //build hashset of previous accesses
        let mut prev_accesses = self
            .prev_accesses
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();

        //store current accesses for next run
        self.prev_accesses.write().unwrap().clone_from(&recorded_accesses);

        let mut new_accesses = HashSet::new();

        for access in recorded_accesses.iter() {
            if !prev_accesses.contains(access) {
                new_accesses.insert(access.clone());
            }
            prev_accesses.remove(access);
        }

        let access_diff = AccessDiff { new_accesses, removed_accesses: prev_accesses };

        let len = outcome.results.len();
        let assertion_outcome = AssertionOutcome::from(outcome);

        trace!(
            len,
            any_assertion_failed = !assertion_outcome.failures.is_empty(),
            "done with assertion results"
        );

        (assertion_outcome, access_diff)
    }

    /// Returns the [`Config`] for the given [`CoreBuildArgs`]
    /// This disables storage caching.
    #[instrument(skip_all)]
    fn get_config(opts: &CoreBuildArgs) -> Config {
        //hack to disable caching, no_storage_caching is not working for forks created using
        // cheatcodes
        Config {
            rpc_storage_caching: StorageCachingConfig {
                endpoints: CachedEndpoints::Pattern(regex::Regex::new("$^").unwrap()),
                ..Default::default()
            },
            no_storage_caching: true,
            ..opts.load_config()
        }
    }

    /// Returns a new [`MultiRunner`] for the given [`Config`].
    #[instrument(skip_all)]
    fn get_multi_runner(
        config: Config,
        project_root: PathBuf,
        output: ProjectCompileOutput,
    ) -> Result<MultiContractRunner, PhylaxRunnerError> {
        let evm_opts = Config::figment().extract::<EvmOpts>().unwrap();
        let evm_spec_id = config.evm_spec_id();

        MultiContractRunnerBuilder::new(Arc::new(config))
            .evm_spec(evm_spec_id)
            .build(&project_root, output, evm_opts.local_evm_env(), evm_opts)
            .map_err(|err| PhylaxRunnerError::RunnerBuildError(format!("{err:#?}")))
    }

    /// Execute the assertions for all contracts.
    #[instrument(skip_all)]
    fn execute_assertion_contracts(
        &self,
        db: Backend,
        tx: mpsc::Sender<(String, (SuiteResult, Vec<Access>))>,
    ) {
        let handle = tokio::runtime::Handle::current();

        trace!("start executing all assertions in project");

        let find_time = std::time::Instant::now();
        let contracts = self.runner.matching_contracts(&self.filter).collect::<Vec<_>>();

        debug!(
            "Found {} assertion contracts out of {} in {find_time:?}",
            contracts.len(),
            self.runner.contracts.len(),
            find_time = find_time.elapsed(),
        );

        // Run assertions for each contract in parallel
        contracts.par_iter().for_each_with(tx, |tx, &(id, contract)| {
            let result = self.execute_assertion_contract(id, contract, db.clone(), &handle);
            let _ = tx.send((id.identifier(), result));
        });
    }

    /// Execute the assertions for a single contract.
    #[instrument(skip_all, fields(contract = get_contract_name(&artifact_id.identifier())))]
    fn execute_assertion_contract(
        &self,
        artifact_id: &ArtifactId,
        contract: &TestContract,
        db: Backend,
        handle: &tokio::runtime::Handle,
    ) -> (SuiteResult, Vec<Access>) {
        let cheats_config = CheatsConfig::new(
            &self.runner.config,
            self.runner.evm_opts.clone(),
            Some(self.runner.known_contracts.clone()),
            None,
            Some(artifact_id.version.clone()),
        );

        let executor = ExecutorBuilder::new()
            .inspectors(|stack| stack.cheatcodes(Arc::new(cheats_config)))
            .spec(self.runner.evm_spec)
            .gas_limit(self.runner.evm_opts.gas_limit())
            .build(self.runner.env.clone(), db);

        debug!("start executing all assertions in contract");

        let id = artifact_id.identifier();

        let mut runner = ContractRunner {
            name: &id,
            executor,
            contract,
            libs_to_deploy: &self.runner.libs_to_deploy,
            initial_balance: self.runner.evm_opts.initial_balance,
            sender: self.runner.sender.unwrap_or_default(),
            revert_decoder: &self.runner.revert_decoder,
            debug: false,
            progress: None,
            tokio_handle: handle,
            span: Span::current(),
        };

        let r =
            runner.run_tests(&self.filter, &self.test_options, self.runner.known_contracts.clone());

        debug!(duration=?r.duration, "executed all assertions in contract");

        (r, runner.executor.get_accesses())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Compile;
    use forge::result::TestStatus;
    use phylax_test_utils::get_alerts_build_args;
    use regex::Regex;

    #[test]
    fn test_get_config() {
        let opts = get_alerts_build_args();

        let config = PhylaxRunner::get_config(&opts);

        assert_eq!(
            config.rpc_storage_caching.endpoints,
            CachedEndpoints::Pattern(regex::Regex::new("$^").unwrap())
        );

        assert!(config.no_storage_caching);
    }

    #[test]
    fn test_get_multi_runner() {
        let filter = FilterArgs {
            contract_pattern: Some(Regex::new("SimpleAlert").unwrap()),
            ..Default::default()
        };

        let build_args = get_alerts_build_args();
        let config = PhylaxRunner::get_config(&build_args);
        let EvmCompilationArtifacts { project_root, output, .. } = build_args.compile().unwrap();

        let multi_runner = PhylaxRunner::get_multi_runner(config, project_root, output).unwrap();

        assert!(multi_runner.matching_contracts(&filter).any(|(id, _)| id.name == "SimpleAlert"));

        let matching_tests = multi_runner.matching_test_functions(&filter).collect::<Vec<_>>();
        assert!(matching_tests.iter().any(|t| t.name == "testBlockNumber"), "no testBlockNumber");
        assert!(matching_tests.iter().any(|t| t.name == "testChainId"), "no testChainId");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_execute_assertion_contracts() {
        let db = Backend::spawn(None);

        let filter = FilterArgs {
            contract_pattern: Some(Regex::new("ForkAlert|ForkStateAlert").unwrap()),
            ..Default::default()
        };
        let runner = PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap();

        let (tx, rx) = channel::<(String, (SuiteResult, Vec<Access>))>();

        tokio::task::spawn_blocking(move || runner.execute_assertion_contracts(db, tx));

        let outcome: Vec<_> = rx.into_iter().collect();

        assert_eq!(outcome.len(), 2);

        let assert_is_valid_outcome = |expected_id: &&str| {
            outcome.iter().any(|(id, (result, _))| {
                id.contains(expected_id) &&
                    !result.test_results.is_empty() &&
                    result.failures().collect::<Vec<_>>().is_empty()
            });
        };

        ["ForkAlert", "ForkStateAlert"].iter().for_each(assert_is_valid_outcome);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_execute_assertion_contract() {
        let db = Backend::spawn(None);

        let filter = FilterArgs {
            contract_pattern: Some(Regex::new("SimpleAlert").unwrap()),
            ..Default::default()
        };

        let runner = PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap();

        let (tx, rx) = mpsc::channel::<(SuiteResult, Vec<Access>)>();

        tokio::task::spawn_blocking(move || {
            let (id, contract) = runner.runner.matching_contracts(&runner.filter).next().unwrap();

            let result = runner.execute_assertion_contract(
                id,
                contract,
                db,
                &tokio::runtime::Handle::current(),
            );
            let _ = tx.send(result);
        });

        let (outcome, _) = rx.recv().unwrap();

        assert_eq!(outcome.test_results.len(), 2);

        let assert_is_valid_test_outcome = |expected_test_id: &&str| {
            assert!(
                outcome.test_results.get(*expected_test_id).unwrap().status == TestStatus::Success
            );
        };

        ["testChainId()", "testBlockNumber()"].iter().for_each(assert_is_valid_test_outcome);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_execute_assertions() {
        let db = Backend::spawn(None);
        let filter = FilterArgs {
            contract_pattern: Some(Regex::new("SimpleAlert").unwrap()),
            ..Default::default()
        };

        let runner = PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap();
        let (outcome, _) = runner.execute_assertions(db);

        assert!(outcome.failures.is_empty());
        assert_eq!(outcome.exports.len(), 2);
    }
}
