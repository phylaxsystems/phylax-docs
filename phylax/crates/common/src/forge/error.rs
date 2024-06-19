use thiserror::Error;

#[derive(Error, Debug)]
pub enum ForgeError {
    #[error("No tests found in the project. Forge looks for functions that start with 'test'")]
    EmptyTests,
    #[error("No tests match the provided pattern {0}")]
    EmptyFilter(String),
}
