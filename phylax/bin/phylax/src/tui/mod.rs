use crossterm::terminal::disable_raw_mode;
use phylax_tracing::tracing::debug;

pub struct Tui;

impl Default for Tui {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        disable_raw_mode().expect("Failed to disable raw mode");
    }
}
impl Tui {
    pub fn new() -> Self {
        Self
    }

    pub async fn detect_ctrl_c(&self) -> eyre::Result<()> {
        let mut stream = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let sigterm = stream.recv();
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::select! {
                _ = ctrl_c => {
                    debug!(target: "phylax::cli",  "Received ctrl-c");
                },
                _ = sigterm => {
                    debug!(target: "phylax::cli",  "Received SIGTERM");
                },
        }
        Ok(())
    }
}
