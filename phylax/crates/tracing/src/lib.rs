//!  The `tracing` module provides functionalities for setting up and configuring logging.
//!
//!  It includes structures and functions to create and manage various logging layers: stdout,
//!  file, or journald. The module's primary entry point is the `PhylaxTracingBuilder` struct, which
//! can be  configured to use different logging formats and destinations. If no layer is specified,
//! it will  default to stdout.
//!
//!  # Examples
//!
//!  Basic usage:
//!
//!  ```
//!  use phylax_tracing::{
//!      error::PhylaxTracingError,
//!      trace::LayerInfo, trace::PhylaxTracingBuilder, trace::Tracer,
//!      tracing::level_filters::LevelFilter,
//!      trace::formatter::LogFormat,
//!  };
//!
//!  fn main() -> Result<(), PhylaxTracingError> {
//!      let tracer = PhylaxTracingBuilder::new().with_stdout(LayerInfo::new(
//!          LogFormat::Json,
//!          LevelFilter::INFO.to_string(),
//!          "debug".to_string(),
//!          None,
//!      ));
//!
//!      tracer.init()?;
//!
//!      // Your application logic here
//!
//!      Ok(())
//!  }
//!  ```
//!
//!  This example sets up a tracer with JSON format logging for journald and terminal-friendly
//! format  for file logging.

#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod error;
pub mod metrics;
pub mod trace;

// Re-export tracing crates
pub use tracing;
pub use tracing_subscriber;
