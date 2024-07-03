//! phylax data directories.
//! Code reused from paradigm/reth reth-node-core crate and licensed under MIT.
// This code is re-licensed and re-distributed by Phylax in the root of this repo.
use crate::utils;
use std::{
    env::VarError,
    fmt::{Debug, Display, Formatter},
    path::{Path, PathBuf},
    str::FromStr,
};

/// Returns the path to the phylax data directory.
///
/// Refer to [dirs_next::data_dir] for cross-platform behavior.
pub fn data_dir() -> Option<PathBuf> {
    dirs_next::data_dir().map(|root| root.join("phylax"))
}

/// Returns the path to the phylax database.
///
/// Refer to [dirs_next::data_dir] for cross-platform behavior.
pub fn database_path() -> Option<PathBuf> {
    data_dir().map(|root| root.join("db"))
}

/// Returns the path to the phylax configuration directory.
///
/// Refer to [dirs_next::config_dir] for cross-platform behavior.
pub fn config_dir() -> Option<PathBuf> {
    dirs_next::config_dir().map(|root| root.join("phylax"))
}

/// Returns the path to the phylax cache directory.
///
/// Refer to [dirs_next::cache_dir] for cross-platform behavior.
pub fn cache_dir() -> Option<PathBuf> {
    dirs_next::cache_dir().map(|root| root.join("phylax"))
}

/// Returns the path to the phylax logs directory.
///
/// Refer to [dirs_next::cache_dir] for cross-platform behavior.
pub fn logs_dir() -> Option<PathBuf> {
    cache_dir().map(|root| root.join("logs"))
}

/// Returns the path to the phylax data dir.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct DataDirPath;

impl XdgPath for DataDirPath {
    fn resolve() -> Option<PathBuf> {
        data_dir()
    }
}

/// Returns the path to the phylax logs directory.
///
/// Refer to [dirs_next::cache_dir] for cross-platform behavior.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct LogsDir;

impl XdgPath for LogsDir {
    fn resolve() -> Option<PathBuf> {
        logs_dir()
    }
}

/// A small helper trait for unit structs that represent a standard path following the XDG
/// path specification.
pub trait XdgPath {
    /// Resolve the standard path.
    fn resolve() -> Option<PathBuf>;
}

/// A wrapper type that either parses a user-given path or defaults to an
/// OS-specific path.
///
/// The [FromStr] implementation supports shell expansions and common patterns such as `~` for the
/// home directory.
///
/// # Example
///
/// ```
/// use phylax_core::dirs::{DataDirPath, PlatformPath};
/// use std::str::FromStr;
///
/// // Resolves to the platform-specific database path
/// let default: PlatformPath<DataDirPath> = PlatformPath::default();
/// // Resolves to `$(pwd)/my/path/to/datadir`
/// let custom: PlatformPath<DataDirPath> = PlatformPath::from_str("my/path/to/datadir").unwrap();
///
/// assert_ne!(default.as_ref(), custom.as_ref());
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct PlatformPath<D>(PathBuf, std::marker::PhantomData<D>);

impl<D> Display for PlatformPath<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl<D> Clone for PlatformPath<D> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), std::marker::PhantomData)
    }
}

impl<D: XdgPath> Default for PlatformPath<D> {
    fn default() -> Self {
        Self(
            D::resolve().expect("Could not resolve default path. Set one manually."),
            std::marker::PhantomData,
        )
    }
}

impl<D> FromStr for PlatformPath<D> {
    type Err = shellexpand::LookupError<VarError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(utils::parse_path(s)?, std::marker::PhantomData))
    }
}

impl<D> AsRef<Path> for PlatformPath<D> {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

impl<D> From<PlatformPath<D>> for PathBuf {
    fn from(value: PlatformPath<D>) -> Self {
        value.0
    }
}

impl<D> PlatformPath<D> {
    /// Returns the path joined with another path
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PlatformPath<D> {
        PlatformPath::<D>(self.0.join(path), std::marker::PhantomData)
    }
}

impl<D> PlatformPath<D> {
    /// Map the inner path to a new type `T`.
    pub fn map_to<T>(&self) -> PlatformPath<T> {
        PlatformPath(self.0.clone(), std::marker::PhantomData)
    }

    /// Returns the path to the phylax data directory
    ///
    /// `<DIR>`
    pub fn data_dir(&self) -> &Path {
        self.0.as_ref()
    }

    /// Returns the path to the db directory
    ///
    /// `<DIR>/db`
    pub fn db(&self) -> PathBuf {
        self.data_dir().join("db")
    }

    /// Returns the path to the config file
    ///
    /// `<DIR>/phylax.yaml`
    pub fn config(&self) -> PathBuf {
        // TODO(Odysseas): remove this once we have a config struct with a const defition of the
        // file type/na
        self.data_dir().join("phylax.yaml")
    }

    /// Returns the path to the jwtsecret file
    ///
    /// `<DIR>/jwt.hex`
    pub fn jwt(&self) -> PathBuf {
        self.data_dir().join("jwt.hex")
    }
}

/// An Optional wrapper type around [PlatformPath].
///
/// This is useful for when a path is optional, such as the `--data-dir` flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaybePlatformPath<D>(Option<PlatformPath<D>>);

// === impl MaybePlatformPath ===

impl<D: XdgPath> MaybePlatformPath<D> {
    /// Returns true if a custom path is set
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// Returns the path if it is set, otherwise returns `None`.
    pub fn as_ref(&self) -> Option<&Path> {
        self.0.as_ref().map(|p| p.as_ref())
    }

    /// Returns the path if it is set
    pub fn unwrap_or_default(&self) -> PlatformPath<D> {
        self.0.clone().unwrap_or_default()
    }
}

impl<D: XdgPath> Display for MaybePlatformPath<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.0 {
            path.fmt(f)
        } else {
            // NOTE: this is a workaround for making it work with clap's `default_value_t` which
            // computes the default value via `Default -> Display -> FromStr`
            write!(f, "default")
        }
    }
}

impl<D> Default for MaybePlatformPath<D> {
    fn default() -> Self {
        Self(None)
    }
}

impl<D> FromStr for MaybePlatformPath<D> {
    type Err = shellexpand::LookupError<VarError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = match s {
            "default" => {
                // NOTE: this is a workaround for making it work with clap's `default_value_t` which
                // computes the default value via `Default -> Display -> FromStr`
                None
            }
            _ => Some(PlatformPath::from_str(s)?),
        };
        Ok(Self(p))
    }
}

impl<D> From<PathBuf> for MaybePlatformPath<D> {
    fn from(path: PathBuf) -> Self {
        Self(Some(PlatformPath(path, std::marker::PhantomData)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maybe_data_dir_path() {
        let path = MaybePlatformPath::<DataDirPath>::default();
        let path = path.unwrap_or_default();
        assert!(path.as_ref().ends_with("phylax"), "{path:?}");

        let db_path = path.db();
        assert!(db_path.ends_with("phylax/db"), "{db_path:?}");

        let path = MaybePlatformPath::<DataDirPath>::from_str("my/path/to/datadir").unwrap();
        let path = path.unwrap_or_default();
        assert!(path.as_ref().ends_with("my/path/to/datadir"), "{path:?}");
    }

    #[test]
    fn test_maybe_testnet_datadir_path() {
        let path = MaybePlatformPath::<DataDirPath>::default();
        let path = path.unwrap_or_default();
        assert!(path.as_ref().ends_with("phylax/"), "{path:?}");
    }
}
