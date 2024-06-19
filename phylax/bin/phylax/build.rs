use std::error::Error;
use vergen::EmitBuilder;

// Code adapted from reth/bin
fn main() -> Result<(), Box<dyn Error>> {
    // Emit the instructions
    EmitBuilder::builder()
        .build_timestamp()
        .cargo_features()
        .cargo_target_triple()
        .git_sha(true)
        .emit()?;
    Ok(())
}
