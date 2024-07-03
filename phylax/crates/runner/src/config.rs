use crate::{error::PhylaxRunnerConfigError, runner::PhylaxRunner};
use forge_cmd::{FilterArgs, ProjectPathsAwareFilter};

use forge::{multi_runner::matches_contract, MultiContractRunnerBuilder};
use foundry_cli::{opts::CoreBuildArgs, utils::LoadConfig};
use foundry_common::{compile::ProjectCompiler, evm::EvmArgs};
use foundry_compilers::{artifacts::output_selection::OutputSelection, utils::source_files_iter};
use foundry_config::{
    figment::{
        self,
        value::{Dict, Map},
        Metadata, Profile, Provider,
    },
    Config,
};

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    result::Result,
    sync::Arc,
};
use phylax_tracing::tracing::{debug, error, trace};

// Loads project's figment and merges the build cli arguments into it
foundry_config::merge_impl_figment_convert!(PhylaxRunnerConfig, opts, evm_args);

/// Phylax Assertion Runner Config.
/// # Example
/// ```rust
/// use phylax_runner::{PhylaxRunnerConfig, FilterArgs, CoreBuildArgs, EvmArgs};
///
/// #[tokio::main]
/// async fn main() {
///    let runner = match PhylaxRunnerConfig::new(CoreBuildArgs::default(), FilterArgs::default()).build() {
///     Ok(runner) => runner,
///    Err(_) => return
///    };
///
///    //Execute assertions against evm block info
///    let results = runner.clone_and_execute_assertions(EvmArgs::default()).await;
///
///    }
/// ```
#[derive(Debug, Clone)]
pub struct PhylaxRunnerConfig {
    /// Filter arguments to filter tests.
    pub filter: FilterArgs,

    /// Core build arguments for source files.
    pub opts: CoreBuildArgs,

    /// Default EVM arguments to construct config with.
    evm_args: EvmArgs,
}

impl PhylaxRunnerConfig {
    /// Creates a new [`PhylaxRunnerConfig`].
    /// # Arguments
    /// * `opts` - Core build arguments for source files.
    /// * `filter` - Filter arguments to filter tests.
    pub fn new(opts: CoreBuildArgs, filter: FilterArgs) -> Self {
        Self { filter, evm_args: Default::default(), opts: CoreBuildArgs { silent: true, ..opts } }
    }

    /// Returns sources which include any tests to be executed.
    /// If no filters are provided, sources are filtered by existence of test methods in
    /// them, If filters are provided, sources are additionaly filtered by them.
    fn get_sources_to_compile(
        &self,
        config: &Config,
        filter: &ProjectPathsAwareFilter,
    ) -> Result<BTreeSet<PathBuf>, PhylaxRunnerConfigError> {
        let mut project = config.create_project(true, true)?;

        project.solc_config.settings.output_selection =
            OutputSelection::common_output_selection(["abi".to_string()]);
        let output = project.compile()?;

        if output.has_compiler_errors() {
            debug!(?output);
            return Err(PhylaxRunnerConfigError::ProjectCompilationFailed(output));
        }

        // ABIs of all sources
        let abis = output
            .into_artifacts()
            .filter_map(|(id, artifact)| artifact.abi.map(|abi| (id, abi)))
            .collect::<BTreeMap<_, _>>();

        // Filter sources by their abis and contract names.
        let mut test_sources = abis
            .iter()
            .filter(|(id, abi)| matches_contract(id, abi, filter))
            .map(|(id, _)| id.source.clone())
            .collect::<BTreeSet<_>>();

        if test_sources.is_empty() {
            error!("No tests found in project! Phylax looks for functions that start with `test`.");
            return Err(PhylaxRunnerConfigError::NoMatchingAssertionsFound);
        }

        // Always recompile all sources to ensure that `getCode` cheatcode can use any artifact.
        test_sources.extend(source_files_iter(project.paths.sources));

        Ok(test_sources)
    }

    /// Compiles the project and returns a [`Result<PhylaxRunner, PhylaxRunnerConfigError>`].
    /// The [`PhylaxRunner`] can be used to execute assertions repetitively using
    /// `PhylaxRunner::clone_and_execute_assertions(EvmArgs { .. })`.
    pub fn build(self) -> Result<PhylaxRunner, PhylaxRunnerConfigError> {
        let (config, evm_opts) = self.load_config_and_evm_opts().map_err(|err| {
            PhylaxRunnerConfigError::LoadConfigAndEvmOptsError(format!("{err:#?}"))
        })?;

        let filter = self.filter.clone().merge_with_config(&config);

        trace!("building phylax runner");

        // Set up the project
        let project = config.project()?;

        let sources_to_compile = self.get_sources_to_compile(&config, &filter)?;

        let compiler = ProjectCompiler::new().quiet_if(true).files(sources_to_compile);
        let output = compiler.compile(&project).map_err(|err| {
            PhylaxRunnerConfigError::AssertionCompilationFailed(format!("{err:#?}"))
        })?;

        let runner_builder = MultiContractRunnerBuilder::new(Arc::new(config.clone()))
            .initial_balance(evm_opts.initial_balance)
            .evm_spec(config.evm_spec_id())
            .sender(evm_opts.sender);

        trace!("done building phylax runner");

        Ok(PhylaxRunner::new(
            runner_builder,
            filter,
            project.paths.root.to_path_buf(),
            output,
            self.opts,
        ))
    }
}

/// [`PhylaxRunnerConfig`] figment provider implementation.
impl Provider for PhylaxRunnerConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named("Core Build Args Provider")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let profile = if let Some(profile) = self.opts.project_paths.profile() {
            profile
        } else {
            Config::selected_profile()
        };
        Ok(Map::from([(profile, Dict::default())]))
    }
}
