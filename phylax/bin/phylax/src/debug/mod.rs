use std::sync::Arc;

use clap::Args;
use phylax_config::PhConfig;
use phylax_core::tasks::spawn::spawn_task_of_config;
use phylax_tracing::tracing::info;
use serde::Serialize;

use crate::tui::Tui;

#[derive(Debug, Clone, Args, Serialize)]
pub struct DebugCommand {
    /// The name of the task to be debugged
    pub task: Vec<String>,
}

impl DebugCommand {
    /// Execute `node` command
    pub async fn execute(self, config: PhConfig) -> eyre::Result<()> {
        let mut artifacts = Vec::new();
        let config_arc = Arc::new(config);
        for task in self.task {
            info!("Debugging [TASK]: {0}", task);
            artifacts.push(spawn_task_of_config(task, config_arc.clone())?);
        }
        let tui = Tui;
        tokio::select! {
            _ = tui.detect_ctrl_c() => {
                info!("Phylax will shutdown now");
                for artifact in artifacts {
                    artifact.handle.shutdown().await?;
                }
                Ok(())
            },
        }
    }
}
