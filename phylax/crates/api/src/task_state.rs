use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum ApiTaskStatus {
    Running { working_fg: bool, working_bg: bool },
    Restarting { reason: String },
    Stopped { reason: Vec<String> },
}
impl ApiTaskStatus {
    /// Returns true if the task is running.
    pub fn is_running(&self) -> bool {
        matches!(self, ApiTaskStatus::Running { .. })
    }

    /// Returns true if the task is restarting.
    pub fn is_restarting(&self) -> bool {
        matches!(self, ApiTaskStatus::Restarting { .. })
    }

    /// Returns true if the task is stopped.
    pub fn is_stopped(&self) -> bool {
        matches!(self, ApiTaskStatus::Stopped { .. })
    }

    /// Returns the working foreground status if the task is running.
    pub fn working_fg(&self) -> Option<bool> {
        if let ApiTaskStatus::Running { working_fg, .. } = self {
            Some(*working_fg)
        } else {
            None
        }
    }

    /// Returns the working background status if the task is running.
    pub fn working_bg(&self) -> Option<bool> {
        if let ApiTaskStatus::Running { working_bg, .. } = self {
            Some(*working_bg)
        } else {
            None
        }
    }

    /// Returns the reason if the task is restarting.
    pub fn restarting_reason(&self) -> Option<&String> {
        if let ApiTaskStatus::Restarting { reason, .. } = self {
            Some(reason)
        } else {
            None
        }
    }

    /// Returns the reason if the task is stopped.
    pub fn stopped_reason(&self) -> Option<&Vec<String>> {
        if let ApiTaskStatus::Stopped { reason, .. } = self {
            Some(reason)
        } else {
            None
        }
    }
}
