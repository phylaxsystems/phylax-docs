use crate::ApiTaskStatus;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
#[schema(value_type=HashMap<String, ApiTaskStatus>)]
pub struct TaskStates(DashMap<String, ApiTaskStatus>);

impl TaskStates {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&self, key: String, value: ApiTaskStatus) -> Option<ApiTaskStatus> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<ApiTaskStatus> {
        self.0.get(key).map(|value| value.clone())
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    // pub fn update_working_status_all(&self, registry: &MetricsRegistry) {
    //     self.0.iter_mut().for_each(|mut item| {
    //         let (name, state) = item.pair_mut();
    //         if let TaskState::Running { working_fg: _, working_bg: _ } = state {
    //             let (fg, bg) = registry.get_activity_state(name);
    //             *state = TaskState::Running { working_fg: fg, working_bg: bg };
    //         }
    //     });
    // }

    // pub fn update_working_status(
    //     &self,
    //     task: &str,
    //     registry: &MetricsRegistry,
    // ) -> Result<(), ApiError> { let mut binding =
    //   self.0.get_mut(task).ok_or(ApiError::UnknownTaskStatus(task.to_owned()))?; let status =
    //   binding.value_mut(); if let TaskState::Running { working_fg: _, working_bg: _ } = status {
    //   let (fg, bg) = registry.get_activity_state(task); *status = TaskState::Running {
    //   working_fg: fg, working_bg: bg }; } Ok(()))
}

impl AsRef<DashMap<String, ApiTaskStatus>> for TaskStates {
    fn as_ref(&self) -> &DashMap<String, ApiTaskStatus> {
        &self.0
    }
}
