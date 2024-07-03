use crate::config::EXTENSION;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid alert name '{0}' in action '{1}'")]
    InvalidAlertName(String, String),
    #[error("Failed to parse TOML: {0}")]
    FailedToParseToml(#[from] toml::de::Error),
    #[error("Failed to read TOML file: {0}")]
    FailedToSerializeToml(#[from] toml::ser::Error),
    #[error("Failed to read TOML file: {0}")]
    FileReadError(std::io::Error),
    #[error("Failed to write TOML file: {0}")]
    FileWriteError(std::io::Error),
    #[error("Wrong config file extension. It must be .{EXTENSION}")]
    WrongConfigFileExtension,
    #[error("Duplicate name '{0}' in {1}[{2}]")]
    DuplicateName(String, String, usize),
}

#[derive(Debug, Error)]
pub enum SubstError {
    #[error("Substitution Error for '{0}': The key '{1}' is malformed")]
    MalformedKey(String, String),
    #[error("Substitution Error: Input is empty")]
    EmptyInput,
}
