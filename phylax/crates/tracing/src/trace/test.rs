// Code reused from paradigm/reth reth-tracing crate and licensed under MIT.
// This code is re-licensed and re-distributed by Phylax in the root of this repo.
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::{error::PhylaxTracingError, trace::Tracer};

///  Initializes a tracing subscriber for tests.
///
///  The filter is configurable via `RUST_LOG`.
///
///  # Note
///
///  The subscriber will silently fail if it could not be installed.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TestTracer;

impl Tracer for TestTracer {
    fn init(self) -> Result<Option<WorkerGuard>, PhylaxTracingError> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .try_init();
        Ok(None)
    }
}
