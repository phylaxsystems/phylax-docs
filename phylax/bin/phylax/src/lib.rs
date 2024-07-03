//! Phylax binary executable.

#![doc(
    html_logo_url = "TODO",
    html_favicon_url = "TODO",
    issue_tracker_base_url = "https://github.com/phylaxsystems/phylax/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod cli;
pub mod commands;

#[cfg(all(unix, any(target_env = "gnu", target_os = "macos")))]
pub mod sigsegv_handler;

/// Signal handler to extract a backtrace from stack overflow.
///
/// This is a no-op because this platform doesn't support our signal handler's requirements.
#[cfg(not(all(unix, any(target_env = "gnu", target_os = "macos"))))]
pub mod sigsegv_handler {
    /// No-op function.
    pub fn install() {}
}
