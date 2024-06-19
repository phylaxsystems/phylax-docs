use eyre::Result;
use futures::Future;
use tokio::runtime::Builder;

pub struct CliRunner {
    runtime: tokio::runtime::Runtime,
}

impl CliRunner {
    pub fn new() -> Self {
        let runtime =
            Builder::new_multi_thread().enable_all().build().expect("Failed to spawn runtime");
        Self { runtime }
    }

    pub fn run_command_until_exit<F, R>(self, command: F) -> Result<R>
    where
        F: Future<Output = Result<R>>,
    {
        self.runtime.block_on(command)
    }
}

impl Default for CliRunner {
    fn default() -> Self {
        Self::new()
    }
}
