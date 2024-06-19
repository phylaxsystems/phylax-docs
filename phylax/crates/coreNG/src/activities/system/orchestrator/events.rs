use serde::{Deserialize, Serialize};
use sisyphus_tasks::sisyphus::TaskStatus;

/// Phylax intenral representation of [`sisyphus_tasks::sisyphus::TaskStatus`]
/// Read more in the sisyphus type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum InternalTaskStatus {
    Running,
    Stopped(bool, Vec<String>),
    Recovering(String),
    Panicked,
    Starting,
}

impl From<TaskStatus> for InternalTaskStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Running => Self::Running,
            TaskStatus::Stopped { exceptional: e, err } => {
                let errors =
                    err.chain().map(|error| clean_ascii(&error.to_string())).collect::<Vec<_>>();
                Self::Stopped(e, errors)
            }
            TaskStatus::Recovering(err) => Self::Recovering(err.to_string()),
            TaskStatus::Panicked => Self::Panicked,
            TaskStatus::Starting => Self::Starting,
        }
    }
}

/// Function to clean ASCII characters from a string.
/// It removes ANSI escape sequences and newline characters.
///
/// # Arguments
///
/// * `msg` - A string slice that holds the message to be cleaned.
///
/// # Returns
///
/// * A String after removing the ANSI escape sequences and newline characters.
fn clean_ascii(msg: &str) -> String {
    let ansi_escape_pattern = regex::Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").unwrap();
    let cleaned = ansi_escape_pattern.replace_all(msg, "").to_string();
    cleaned.replace('\n', "")
}

/// The [`crate::events::Event`] body for the events of type
/// [`crate::events::EventType::TaskStatusChange`] Eventually, all event bodies should implement
/// some common trait so that we can easily group them in the codebase. It should be easily
/// discoverable for the tasks and the developpers to find what event bodies exists and what event
/// bodies are usually found with which event types
#[derive(Serialize, Deserialize, Clone)]
pub struct TaskStatusEventBody {
    /// The name of the task
    pub task: String,
    /// The task status
    pub state: InternalTaskStatus,
}

impl TaskStatusEventBody {
    /// Create a new event body
    pub fn new(task: String, state: impl Into<InternalTaskStatus>) -> Self {
        Self { task, state: state.into() }
    }

    /// Convert the [`TaskStatusEventBody`] to [`serde_json::Value`]
    /// This is used so that it can be sent in the event bus
    pub fn as_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eyre::Report;

    use super::*;

    #[test]
    fn test_internal_task_status_from() {
        let task_status = TaskStatus::Running;
        let internal_status = InternalTaskStatus::from(task_status.clone());
        assert_eq!(internal_status, InternalTaskStatus::Running);

        let task_status =
            TaskStatus::Stopped { exceptional: true, err: Arc::new(Report::msg("yolo")) };
        let internal_status = InternalTaskStatus::from(task_status.clone());
        assert_eq!(internal_status, InternalTaskStatus::Stopped(true, vec!["yolo".to_string()]));

        // Add more tests for other variants of TaskStatus
    }

    #[test]
    fn test_clean_ascii() {
        let msg = "Hello\x1B[0m\nWorld";
        let cleaned = clean_ascii(msg);
        assert_eq!(cleaned, "HelloWorld");
    }

    #[test]
    fn test_task_status_event_body_new() {
        let task = "test_task".to_string();
        let state = InternalTaskStatus::Running;
        let event_body = TaskStatusEventBody::new(task.clone(), state.clone());
        assert_eq!(event_body.task, task);
        assert_eq!(event_body.state, state);
    }

    #[test]
    fn test_task_status_event_body_as_value() {
        let task = "test_task".to_string();
        let state = InternalTaskStatus::Running;
        let event_body = TaskStatusEventBody::new(task.clone(), state.clone());
        let value = event_body.as_value();
        assert_eq!(value["task"], task);
        assert_eq!(value["state"], "Running");
    }
}
