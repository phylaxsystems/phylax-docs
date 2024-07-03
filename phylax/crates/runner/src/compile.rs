use crate::error::EvmCompilationError;
use forge_cmd::{FilterArgs, ProjectPathsAwareFilter};

use forge::multi_runner::matches_contract;
use foundry_cli::{opts::CoreBuildArgs, utils::LoadConfig};
use foundry_common::compile::ProjectCompiler;
use foundry_compilers::{
    artifacts::output_selection::OutputSelection,
    utils::{source_files_iter, SOLC_EXTENSIONS},
    ProjectCompileOutput,
};
use foundry_config::Config;

use phylax_tracing::tracing::{self, error, instrument, trace};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    result::Result,
};

/// Compilation artifacts for EVM smart-contract project.
#[derive(Debug, Clone)]
pub struct EvmCompilationArtifacts {
    /// Compilation output.
    pub output: ProjectCompileOutput,
    /// Root directory of the project.
    pub project_root: PathBuf,
    /// CoreBuildArgs used to compile the project.
    pub opts: CoreBuildArgs,
}

/// Trait for compiling smart-contracts into given artifacts.
pub trait Compile<Artifacts, E> {
    /// Trait method for compiling [`self`] into generic Artifacts
    fn compile(self) -> Result<Artifacts, E>;
}

/// Implementation of [`Compile`] trait for [`CoreBuildArgs`].
/// Compiles smart-contracts into EvmCompilationArtifacts.
/// If no tests are found in the project, returns an error.
impl Compile<EvmCompilationArtifacts, EvmCompilationError> for CoreBuildArgs {
    #[instrument(skip_all, fields(project_root = ?self.project_paths.root))]
    fn compile(self) -> Result<EvmCompilationArtifacts, EvmCompilationError> {
        let config = self.load_config();

        let filter = FilterArgs::default().merge_with_config(&config);

        trace!("Compiling CoreBuildArgs into EvmCompilationArtifacts");

        let sources_to_compile = get_sources_to_compile(&config, &filter)?;

        let compiler = ProjectCompiler::new().quiet_if(true).files(sources_to_compile);

        let project = config.ephemeral_no_artifacts_project()?;
        let output = compiler
            .compile(&project)
            .map_err(|err| EvmCompilationError::AssertionCompilationFailed(format!("{err:#?}")))?;

        trace!("Done compiling CoreBuildArgs into EvmCompilationArtifacts");

        Ok(EvmCompilationArtifacts { output, project_root: project.paths.root, opts: self })
    }
}

/// Returns sources which include any tests to be executed.
/// If no filters are provided, sources are filtered by existence of test methods in
/// them, If filters are provided, sources are additionaly filtered by them.
fn get_sources_to_compile(
    config: &Config,
    filter: &ProjectPathsAwareFilter,
) -> Result<BTreeSet<PathBuf>, EvmCompilationError> {
    let mut project = config.ephemeral_no_artifacts_project()?;
    project.settings.solc.output_selection =
        OutputSelection::common_output_selection(["abi".to_string()]);

    let output = project.compile()?;

    if output.has_compiler_errors() {
        error!(?output, "Project compilation failed");
        return Err(EvmCompilationError::ProjectCompilationFailed(output));
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
        return Err(EvmCompilationError::NoMatchingAssertionsFound);
    }

    // Always recompile all sources to ensure that `getCode` cheatcode can use any artifact.
    test_sources.extend(source_files_iter(project.paths.sources, SOLC_EXTENSIONS));

    Ok(test_sources)
}

#[cfg(test)]
mod tests {
    use super::*;
    use phylax_test_utils::{get_alerts_build_args, ALERTS_BASE_PATH};
    use regex::Regex;
    use std::path::Path;

    #[test]
    fn test_file_storage_not_used() {
        get_alerts_build_args().compile().unwrap();

        assert!(!Path::new(&format!("{ALERTS_BASE_PATH}/out")).is_dir());
        assert!(!Path::new(&format!("{ALERTS_BASE_PATH}/cache")).is_dir());
    }

    #[test]
    fn test_get_sources_to_compile_no_tests_found() {
        let build_args = get_alerts_build_args();
        let config = build_args.load_config();
        let filter = FilterArgs {
            contract_pattern: Some(Regex::new("DoesNotExist").unwrap()),
            ..Default::default()
        }
        .merge_with_config(&config);

        assert!(matches!(
            get_sources_to_compile(&config, &filter).unwrap_err(),
            EvmCompilationError::NoMatchingAssertionsFound
        ));
    }

    #[test]
    fn test_get_sources_to_compile_alerts() {
        let build_args = get_alerts_build_args();

        let config = build_args.load_config();
        let filter = FilterArgs {
            contract_pattern: Some(Regex::new("SimpleAlert").unwrap()),
            ..Default::default()
        }
        .merge_with_config(&config);

        let sources = {
            let mut result = get_sources_to_compile(&config, &filter);
            while result.is_err() {
                result = get_sources_to_compile(&config, &filter);
            }
            result.unwrap()
        };

        assert!(!sources.is_empty());
    }
}
