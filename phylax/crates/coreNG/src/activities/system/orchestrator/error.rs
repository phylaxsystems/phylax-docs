use thiserror::Error;

use crate::{activities::ActivityError, tasks::error::SpawnError};

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("Failed to spawn tasks")]
    SpawnTasks(#[from] SpawnError),
    #[error("Orchestrator failed to send shutdown signal to task")]
    TaskShutdownSignal,
    #[error("Orchestrator's Task Controller failed to send callback signal")]
    TaskShutdownCallback,
}

#[derive(Debug, Error)]
pub enum TaskManagerError {
    #[error("Task Manager siganl that all tasks have been shutdown")]
    TaskShutdownCallback,
    #[error("Task Manager failed to send a new task status for [Task | id: {0}]")]
    TaskStatusSend(u32),
}

impl From<OrchestratorError> for ActivityError {
    fn from(error: OrchestratorError) -> Self {
        ActivityError::BespokeError(Box::new(error))
    }
}
