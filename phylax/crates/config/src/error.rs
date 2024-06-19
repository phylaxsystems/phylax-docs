use phylax_common::SubstError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to substitute key in the config")]
    EnvVarSub {
        #[from]
        source: SubstError,
    },
}
