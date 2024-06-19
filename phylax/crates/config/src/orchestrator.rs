use crate::{PhConfig, Trigger, API_TASK_NAME};

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestratorConfig {
    pub on: Vec<Trigger>,
}
impl OrchestratorConfig {
    /// The orchestrator is subscribed to no events
    fn default_triggers() -> Vec<Trigger> {
        Vec::new()
    }
    /// Create triggers based on the api config and config watcher flag of PhConfig
    /// If the api task is not enabled, there is no reason to subscribe to their
    /// events and thus check events for these rules.
    pub(crate) fn api_or_metrics_triggers(ph_config: &PhConfig) -> Vec<Trigger> {
        let mut triggers = Vec::new();
        if ph_config.api.enable_api {
            let api_trigger =
                Trigger { rule: format!("origin == hash \"{}\"", API_TASK_NAME), interval: 1 };
            triggers.push(api_trigger);
        }
        triggers
    }
}
impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self { on: Self::default_triggers() }
    }
}
