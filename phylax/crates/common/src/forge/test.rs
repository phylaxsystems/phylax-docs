use alloy_primitives::U256;
use clap::Parser;
use color_eyre::eyre;
use eyre::Result;
use forge::{
    executor::inspector::CheatsConfig,
    result::{SuiteResult, TestResult, TestStatus},
    trace::identifier::LocalTraceIdentifier,
    MultiContractRunner, MultiContractRunnerBuilder, TestOptions, TestOptionsBuilder,
};
use foundry_cli::{opts::CoreBuildArgs, utils::LoadConfig};
use foundry_common::{
    compile::{self, ProjectCompiler},
    evm::EvmArgs,
    get_contract_name, get_file_name, shell,
};

use foundry_config::{
    figment,
    figment::{
        value::{Dict, Map},
        Metadata, Profile, Provider,
    },
    get_available_profiles, Config,
};
use foundry_evm::revm::primitives::alloy_primitives;
use phylax_tracing::tracing::trace;
use regex::Regex;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use super::{
    error::ForgeError,
    filter::{FilterArgs, ProjectPathsAwareFilter},
};

// Loads project's figment and merges the build cli arguments into it
foundry_config::merge_impl_figment_convert!(TestArgs, opts, evm_opts);

/// CLI arguments for `forge test`.
#[derive(Debug, Clone, Parser, Default)]
#[clap(next_help_heading = "Test options")]
pub struct TestArgs {
    /// Run a test in the debugger.
    ///
    /// The argument passed to this flag is the name of the test function you want to run, and it
    /// works the same as --match-test.
    ///
    /// If more than one test matches your specified criteria, you must add additional filters
    /// until only one test is found (see --match-contract and --match-path).
    ///
    /// The matching test will be opened in the debugger regardless of the outcome of the test.
    ///
    /// If the matching test is a fuzz test, then it will open the debugger on the first failure
    /// case.
    /// If the fuzz test does not fail, it will open the debugger on the last fuzz case.
    ///
    /// For more fine-grained control of which fuzz case is run, see forge run.
    #[clap(long, value_name = "TEST_FUNCTION")]
    pub debug: Option<Regex>,

    /// Exit with code 0 even if a test fails.
    #[clap(long, env = "FORGE_ALLOW_FAILURE")]
    pub allow_failure: bool,

    #[clap(flatten)]
    pub filter: FilterArgs,

    /// Output test results in JSON format.
    #[clap(long, short, help_heading = "Display options")]
    pub json: bool,

    /// Stop running tests after the first failure
    #[clap(long)]
    pub fail_fast: bool,

    /// The Etherscan (or equivalent) API key
    #[clap(long, env = "ETHERSCAN_API_KEY", value_name = "KEY")]
    pub etherscan_api_key: Option<String>,

    /// Set seed used to generate randomness during your fuzz runs.
    #[clap(long)]
    pub fuzz_seed: Option<U256>,

    #[clap(long, env = "FOUNDRY_FUZZ_RUNS", value_name = "RUNS")]
    pub fuzz_runs: Option<u64>,

    #[clap(flatten)]
    pub evm_opts: EvmArgs,

    #[clap(flatten)]
    pub opts: CoreBuildArgs,

    /// Print test summary table
    #[clap(long, help_heading = "Display options")]
    pub summary: bool,

    /// Print detailed test summary table
    #[clap(long, help_heading = "Display options")]
    pub detailed: bool,
}

impl TestArgs {
    /// Returns the flattened [`CoreBuildArgs`].
    pub fn build_args(&self) -> &CoreBuildArgs {
        &self.opts
    }

    pub async fn run(self, context_map: HashMap<String, Value>) -> Result<TestOutcome> {
        trace!(target: "forge::test", "executing test command");
        shell::set_shell(shell::Shell::from_args(self.opts.silent, self.json))?;
        self.execute_tests(context_map).await
    }

    /// Executes all the tests in the project.
    ///
    /// This will trigger the build process first. On success all test contracts that match the
    /// configured filter will be executed
    ///
    /// Returns the test results for all matching tests.
    pub async fn execute_tests(self, context: HashMap<String, Value>) -> Result<TestOutcome> {
        // Merge all configs
        let (config, evm_opts) = self.load_config_and_evm_opts()?;

        let filter = self.filter(&config);

        trace!(target: "forge::test", ?filter, "using filter");

        // Set up the project
        let project = config.project()?;

        let compiler = ProjectCompiler::default();
        let output = match (config.sparse_mode, self.opts.silent | self.json) {
            (false, false) => compiler.compile(&project),
            (true, false) => compiler.compile_sparse(&project, filter.clone()),
            (false, true) => compile::suppress_compile(&project),
            (true, true) => compile::suppress_compile_sparse(&project, filter.clone()),
        }?;
        // Create test options from general project settings
        // and compiler output
        let project_root = &project.paths.root;
        let toml = config.get_config_path();
        let profiles = get_available_profiles(toml)?;

        let test_options: TestOptions = TestOptionsBuilder::default()
            .fuzz(config.fuzz)
            .invariant(config.invariant)
            .profiles(profiles)
            .build(&output, project_root)?;

        let env = evm_opts.evm_env().await?;

        let runner_builder = MultiContractRunnerBuilder::default()
            .initial_balance(evm_opts.initial_balance)
            .evm_spec(config.evm_spec_id())
            .sender(evm_opts.sender)
            .with_fork(evm_opts.get_fork(&config, env.clone()))
            .with_cheats_config(CheatsConfig::new(&config, &evm_opts))
            .with_test_options(test_options.clone());

        let runner = runner_builder.clone().build(
            project_root,
            output.clone(),
            env.clone(),
            evm_opts.clone(),
        )?;

        let known_contracts = runner.known_contracts.clone();
        let _local_identifier = LocalTraceIdentifier::new(&known_contracts);
        let _remote_chain_id = runner.evm_opts.get_remote_chain_id();

        let outcome = self.run_tests(runner, filter.clone(), test_options.clone(), context).await?;

        Ok(outcome)
    }

    /// Run all tests that matches the filter predicate from a test runner
    pub async fn run_tests(
        &self,
        runner: MultiContractRunner,
        filter: ProjectPathsAwareFilter,
        test_options: TestOptions,
        context: HashMap<String, Value>,
    ) -> Result<TestOutcome, ForgeError> {
        test(runner, filter, test_options, context).await
    }

    /// Returns the flattened [`FilterArgs`] arguments merged with [`Config`].
    pub fn filter(&self, config: &Config) -> ProjectPathsAwareFilter {
        self.filter.merge_with_config(config)
    }
}

impl Provider for TestArgs {
    fn metadata(&self) -> Metadata {
        Metadata::named("Core Build Args Provider")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let mut dict = Dict::default();

        let mut fuzz_dict = Dict::default();
        if let Some(fuzz_seed) = self.fuzz_seed {
            fuzz_dict.insert("seed".to_string(), fuzz_seed.to_string().into());
        }
        if let Some(fuzz_runs) = self.fuzz_runs {
            fuzz_dict.insert("runs".to_string(), fuzz_runs.into());
        }
        dict.insert("fuzz".to_string(), fuzz_dict.into());

        if let Some(ref etherscan_api_key) = self.etherscan_api_key {
            dict.insert("etherscan_api_key".to_string(), etherscan_api_key.to_string().into());
        }
        let profile = if let Some(profile) = self.build_args().project_paths.profile.as_ref() {
            profile.into()
        } else {
            Config::selected_profile()
        };
        Ok(Map::from([(profile, dict)]))
    }
}

/// The result of a single test
#[derive(Debug, Clone)]
pub struct Test {
    /// The identifier of the artifact/contract in the form of `<artifact file name>:<contract
    /// name>`
    pub artifact_id: String,
    /// The signature of the solidity test
    pub signature: String,
    /// Result of the executed solidity test
    pub result: TestResult,
}

impl Test {
    pub fn gas_used(&self) -> u64 {
        self.result.kind.report().gas()
    }

    /// Returns the contract name of the artifact id
    pub fn contract_name(&self) -> &str {
        get_contract_name(&self.artifact_id)
    }

    /// Returns the file name of the artifact id
    pub fn file_name(&self) -> &str {
        get_file_name(&self.artifact_id)
    }
}

/// Represents the bundled results of all tests
#[derive(Clone, Debug)]
pub struct TestOutcome {
    /// Whether failures are allowed
    pub allow_failure: bool,
    /// Results for each suite of tests `contract -> SuiteResult`
    pub results: BTreeMap<String, SuiteResult>,
}

impl TestOutcome {
    fn new(results: BTreeMap<String, SuiteResult>, allow_failure: bool) -> Self {
        Self { results, allow_failure }
    }

    /// Iterator over all succeeding tests and their names
    pub fn successes(&self) -> impl Iterator<Item = (&String, &TestResult)> {
        self.tests().filter(|(_, t)| t.status == TestStatus::Success)
    }

    /// Iterator over all failing tests and their names
    pub fn failures(&self) -> impl Iterator<Item = (&String, &TestResult)> {
        self.tests().filter(|(_, t)| t.status == TestStatus::Failure)
    }

    pub fn skips(&self) -> impl Iterator<Item = (&String, &TestResult)> {
        self.tests().filter(|(_, t)| t.status == TestStatus::Skipped)
    }

    /// Iterator over all tests and their names
    pub fn tests(&self) -> impl Iterator<Item = (&String, &TestResult)> {
        self.results.values().flat_map(|suite| suite.tests())
    }

    /// Returns an iterator over all `Test`
    pub fn into_tests(self) -> impl Iterator<Item = Test> {
        self.results
            .into_iter()
            .flat_map(|(file, SuiteResult { test_results, .. })| {
                test_results.into_iter().map(move |t| (file.clone(), t))
            })
            .map(|(artifact_id, (signature, result))| Test { artifact_id, signature, result })
    }

    pub fn duration(&self) -> Duration {
        self.results
            .values()
            .fold(Duration::ZERO, |acc, SuiteResult { duration, .. }| acc + *duration)
    }
}

/// Runs all the tests
#[allow(clippy::too_many_arguments)]
async fn test(
    mut runner: MultiContractRunner,
    filter: ProjectPathsAwareFilter,
    test_options: TestOptions,
    context: HashMap<String, Value>,
) -> Result<TestOutcome, ForgeError> {
    trace!(target: "forge::test", "running all tests");
    if runner.matching_test_function_count(&filter) == 0 {
        let filter_str = filter.to_string();
        if filter_str.is_empty() {
            return Err(ForgeError::EmptyTests);
        } else {
            return Err(ForgeError::EmptyFilter(filter_str));
        }
    }
    let results = runner.test(filter, None, test_options, Some(context)).await;
    let _num_test_suites = results.len();
    trace!(target: "forge::test", "received {} results", results.len());
    Ok(TestOutcome::new(results, false))
}
