use foundry_compilers::{error::SolcError, ProjectCompileOutput};
use foundry_config::InlineConfigError;

use thiserror::Error;

/// Error originating from evm compilation in [`crate::compile`].
#[derive(Error, Debug)]
pub enum EvmCompilationError {
    ///No assertions found in the project matching the provided filters.
    #[error("No assertions found in the project matching the provided filters.")]
    NoMatchingAssertionsFound,
    ///Failed to compile the project.
    #[error("Failed to create project: {0}")]
    ProjectCompilationFailed(ProjectCompileOutput),
    ///Failed to compile an assertion.
    #[error("Compilation failed: {0}")]
    AssertionCompilationFailed(String),
    ///Failed to create the project.
    #[error("Failed to create project: {0}")]
    FailedToCreateProject(#[from] SolcError),
}

/// Error originating from the [PhylaxRunner][`crate::PhylaxRunner`].
#[derive(Debug, Error)]
pub enum PhylaxRunnerError {
    ///Runner Builder Error.
    #[error("Runner build error: {0}")]
    RunnerBuildError(String),
    ///Inline Config Error.
    #[error("Inline config error: {0}")]
    InlineConfigError(#[from] InlineConfigError),
}
