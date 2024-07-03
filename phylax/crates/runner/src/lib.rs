//! This crate is home to the [`PhylaxRunner`] for running assertions, and the [`Compile`] trait
//! for compiling smart contracts.

mod accesses;
mod compile;
mod error;
mod outcome;
mod runner;

pub use compile::{Compile, EvmCompilationArtifacts};

pub use runner::PhylaxRunner;

pub use error::{EvmCompilationError, PhylaxRunnerError};

pub use outcome::{AssertionOutcome, PhylaxExport};

/// Forge's filter args, re-exported for convenience.
pub use forge_cmd::FilterArgs;

/// Foundry's core build args, re-exported for convenience.
pub use foundry_cli::opts::{CoreBuildArgs, ProjectPathsArgs};

/// Forge's Backend, re-exported for convenience.
pub use forge::backend::Backend;

pub use accesses::DataAccesses;

#[cfg(test)]
mod test {
    use foundry_evm::backend::{Access, AccessType, RevmDbAccess, StateLookup};
    pub(crate) fn get_access() -> Access {
        Access {
            chain: 1.into(),
            access_type: AccessType::RevmDbAccess(RevmDbAccess::Storage(
                Default::default(),
                Default::default(),
            )),
            state_lookup: StateLookup::default(),
        }
    }
}
