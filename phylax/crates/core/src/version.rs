//! Version information for phylax.
//! Adapted from https://github.com/paradigmxyz/reth/blob/e6d0d7e00b3a6469b6983f4f4f3f9c9e85637bcd/crates/node-core/src/version.rs

/// The latest version from Cargo.toml.
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The short SHA of the latest commit.
pub const VERGEN_GIT_SHA: &str = env!("VERGEN_GIT_SHA");

/// The build timestamp.
pub const VERGEN_BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");

/// The short version information for phylax.
///
/// - The latest version from Cargo.toml
/// - The short SHA of the latest commit.
///
/// # Example
///
/// ```text
/// 0.1.0 (defa64b2)
/// ```
pub const SHORT_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    env!("PHYLAX_VERSION_SUFFIX"),
    " (",
    env!("VERGEN_GIT_SHA"),
    ")"
);

/// The long version information for phylax.
///
/// - The latest version from Cargo.toml
/// - The long SHA of the latest commit.
/// - The build datetime
/// - The build features
/// - The build profile
///
/// # Example:
///
/// ```text
/// Version: 0.1.0
/// Commit SHA: defa64b2
/// Build Timestamp: 2023-05-19T01:47:19.815651705Z
/// Build Profile: maxperf
/// ```
pub const LONG_VERSION: &str = const_str::concat!(
    "Version: ",
    env!("CARGO_PKG_VERSION"),
    env!("PHYLAX_VERSION_SUFFIX"),
    "\n",
    "Commit SHA: ",
    env!("VERGEN_GIT_SHA"),
    "\n",
    "Build Timestamp: ",
    env!("VERGEN_BUILD_TIMESTAMP"),
    "\n",
    "Build Features: ",
    env!("VERGEN_CARGO_FEATURES"),
    "\n",
    "Build Profile: ",
    build_profile_name()
);

/// The default extradata used for payload building.
///
/// - The latest version from Cargo.toml
/// - The OS identifier
///
/// # Example
///
/// ```text
/// phylax/v{major}.{minor}.{patch}/{OS}
/// ```
pub fn default_extradata() -> String {
    format!("phylax/v{}/{}", env!("CARGO_PKG_VERSION"), std::env::consts::OS)
}

/// The default phylax version accessing the database.
pub fn default_client_version() -> String {
    "TODO Database versioning".to_string()
}

pub(crate) const fn build_profile_name() -> &'static str {
    // Derived from https://stackoverflow.com/questions/73595435/how-to-get-profile-from-cargo-toml-in-build-rs-or-at-runtime
    // We split on the path separator of the *host* machine, which may be different from
    // `std::path::MAIN_SEPARATOR_STR`.
    const OUT_DIR: &str = env!("OUT_DIR");
    const SEP: char = if const_str::contains!(OUT_DIR, "/") { '/' } else { '\\' };
    let parts = const_str::split!(OUT_DIR, SEP);
    parts[parts.len() - 4]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_extradata_less_32bytes() {
        let extradata = default_extradata();
        assert!(
            extradata.as_bytes().len() <= 32,
            "extradata must be less than 32 bytes: {extradata}"
        )
    }
}
