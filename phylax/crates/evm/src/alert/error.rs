use phylax_interfaces::error::PhylaxError;

///Typed Errors for [`EvmAlert`]
#[derive(thiserror::Error, Debug)]
pub enum EvmAlertError {
    ///Poller did not yield block hash
    #[error("{0} monitor not found.")]
    MonitorNotFound(String),
    ///Type not found in state registry
    #[error("{0} not found in StateRegistry.")]
    TypeNotFoundInStateRegistry(String),
}

#[allow(clippy::from_over_into)]
impl Into<PhylaxError> for EvmAlertError {
    fn into(self) -> PhylaxError {
        PhylaxError::UnrecoverableError(Box::new(self))
    }
}
