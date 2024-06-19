use eyre::Result;
use phylax_tracing::tracing::debug;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use dashmap::DashMap;

/// `TaskDns` is a structure that maps task names to their hashed ids.
/// Internally, it uses [`DashMap`] so that we can put it in an Arc and pass it around
/// and tasks can read/write without having to deal with concurency
#[derive(Default, Debug)]
pub struct TaskDns(DashMap<u32, String>);

impl TaskDns {
    /// Creates a new `TaskDns`.
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    /// Returns the hashed id of a given task name.
    pub fn get_id_from_name(&self, name: &str) -> u32 {
        hash_string(name)
    }

    /// Finds the hashed id of a given task name.
    /// Returns `None` if the task name is not found.
    pub fn find_id_from_name(&self, name: &str) -> Option<u32> {
        debug!(name, "Retrieving a task id from the DNS");
        let id = hash_string(name);
        if self.0.get(&id).is_some() {
            Some(id)
        } else {
            phylax_tracing::tracing::warn!(
                target = "ph_core::orchestrator::TaskDns",
                "TaskDns: No task id found for task name: {}",
                id
            );
            None
        }
    }

    /// Finds the task name from a given hashed id.
    /// Returns `None` if the id is not found.
    pub fn find_name_from_id(&self, id: u32) -> Option<String> {
        let res = self.0.get(&id).map(|id| id.clone());
        debug!(id, "Retrieving a task name from the DNS");
        match res {
            None => {
                phylax_tracing::tracing::warn!(
                    target = "ph_core::orchestrator::TaskDns",
                    "TaskDns: No task name found for id: {}",
                    id
                );
                None
            }
            Some(name) => Some(name),
        }
    }

    /// Stores a task name and its hashed id in the `TaskDns`.
    /// Returns `Ok(())` if the task name is stored successfully.
    /// Returns `Err` if the task name is a duplicate.
    pub fn store_name(&self, name: &str) -> Result<()> {
        let id = hash_string(name);
        debug!(id, name, "Storing a task name in the DNS");
        match self.0.insert(id, name.to_string()) {
            None => Ok(()),
            Some(_) => Err(eyre::eyre!("Duplicate task name: {}", name)),
        }
    }
}

/// Hashes a string using `DefaultHasher`.
/// Returns the hashed value as `u32`.
pub fn hash_string(s: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let dns = TaskDns::new();
        assert!(dns.0.is_empty());
    }

    #[test]
    fn test_get_id_from_name() {
        let dns = TaskDns::new();
        let name = "test";
        let id = hash_string(name);
        assert_eq!(dns.get_id_from_name(name), id);
    }

    #[test]
    fn test_find_id_from_name() {
        let dns = TaskDns::new();
        let name = "test";
        let id = hash_string(name);
        dns.0.insert(id, name.to_string());
        assert_eq!(dns.find_id_from_name(name), Some(id));
    }

    #[test]
    fn test_find_name_from_id() {
        let dns = TaskDns::new();
        let name = "test";
        let id = hash_string(name);
        dns.0.insert(id, name.to_string());
        assert_eq!(dns.find_name_from_id(id), Some(name.to_string()));
    }

    #[test]
    fn test_store_name() {
        let dns = TaskDns::new();
        let name = "test";
        assert!(dns.store_name(name).is_ok());
        assert!(dns.store_name(name).is_err());
    }

    #[test]
    fn test_hash_string() {
        let s = "test";
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        let expected = hasher.finish() as u32;
        assert_eq!(hash_string(s), expected);
    }
}
