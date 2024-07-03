use tracing_subscriber::filter::ParseError;

/// An error kind which covers potential error conversions from logic in the
/// cache or underlying state provider implementations.
#[derive(thiserror::Error, Debug)]
pub enum PhylaxTracingError {
    /// Error thrown when parsing strings into directive names
    #[error("Directive name could not be parsed from the provided string: {0:?}")]
    DirectiveParseError(#[from] ParseError),
    /// Error thrown when initializing telemetry, such as when attempting to setup IO
    /// or when initializing an OpenTelemtry provider.
    #[error("Unexpected error when initializing telemetry: {0:?}")]
    InitError(#[from] eyre::Report),
}
