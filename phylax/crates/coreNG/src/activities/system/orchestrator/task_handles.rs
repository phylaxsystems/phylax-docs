use sisyphus_tasks::Sisyphus;
use std::{collections::BTreeMap, fmt};

/// A mapping between Task IDs and [`sisyphus::Sisyphus`] handles
#[derive(Default)]
pub struct TaskHandles(pub(crate) BTreeMap<u32, Sisyphus>);

/// Implementation of TaskHandles
impl TaskHandles {
    /// Create a new instance of TaskHandles
    pub fn new() -> Self {
        Default::default()
    }

    /// Insert a new task with a given id into the TaskHandles
    pub fn insert(&mut self, id: u32, task: Sisyphus) {
        self.0.insert(id, task);
    }

    /// Get a task by its id from the TaskHandles
    pub fn get(&self, id: &u32) -> Option<&Sisyphus> {
        self.0.get(id)
    }

    /// Remove and return the first task from the TaskHandles
    pub fn pop_first(&mut self) -> Option<(u32, Sisyphus)> {
        self.0.pop_first()
    }
    /// Returns the number of tasks in the TaskHandles
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for TaskHandles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TaskHandles {{ tasks }}")
    }
}
