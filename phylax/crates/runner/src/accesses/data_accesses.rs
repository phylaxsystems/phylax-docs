use super::AccessDiff;
use dashmap::DashMap;
use foundry_evm::backend::{Access, Backend, DatabaseError};

use alloy_chains::Chain;

/// A struct meant for tracking shared data accesses for many runners
#[derive(Clone, Default, Debug)]
pub struct DataAccesses {
    ///Unique data access keys by the count of their dependent runners
    pub data_accesses: DashMap<Access, u32>,
}

impl DataAccesses {
    /// Increment and decrement the data accesses according to the consumed ['AccessDiff']
    pub fn apply_access_diff(&self, AccessDiff { new_accesses, removed_accesses }: AccessDiff) {
        for new_access in new_accesses {
            self.data_accesses.entry(new_access).and_modify(|val| *val += 1).or_insert(1);
        }

        for removed_access in removed_accesses {
            let val = self
                .data_accesses
                .entry(removed_access.clone())
                .and_modify(|val| *val -= 1)
                .or_default();
            if *val == 0 {
                //Avoids a dead-lock on the DashMap
                std::mem::drop(val);
                self.data_accesses.remove(&removed_access);
            }
        }
    }

    /// Get the data accesses for a chain
    pub fn get_data_accesses(&self, chain: Chain) -> Vec<Access> {
        self.data_accesses
            .clone()
            .into_read_only()
            .keys()
            .filter(|access| access.chain == chain)
            .cloned()
            .collect()
    }

    /// Load the data accesses into the database
    pub fn load_data_accesses(
        &self,
        db: &Backend,
        chain: Chain,
        block_number: u64,
        url: String,
    ) -> Result<(), DatabaseError> {
        db.load_accesses(&self.get_data_accesses(chain), chain, block_number, url)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_apply_access_diff_counter_rm_at_0() {
    use crate::test::get_access;
    let data_accesses = DataAccesses { data_accesses: DashMap::new() };

    let access = get_access();

    data_accesses.data_accesses.insert(access.clone(), 1);

    let access_diff = AccessDiff {
        new_accesses: Default::default(),
        removed_accesses: vec![access].into_iter().collect(),
    };

    data_accesses.apply_access_diff(access_diff);

    assert!(data_accesses.get_data_accesses(Chain::default()).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_apply_access_diff_counter() {
    use crate::test::get_access;
    let data_accesses = DataAccesses { data_accesses: DashMap::new() };

    let access = get_access();

    data_accesses.data_accesses.insert(access.clone(), 1);

    let acess_diff = AccessDiff {
        new_accesses: vec![access.clone()].into_iter().collect(),
        removed_accesses: vec![access.clone()].into_iter().collect(),
    };

    data_accesses.apply_access_diff(acess_diff);

    assert!(data_accesses.get_data_accesses(Chain::default()) == vec![access]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_data_accesses_underflow_safe() {
    use crate::test::get_access;
    let data_accesses = DataAccesses { data_accesses: DashMap::new() };

    let access = get_access();

    data_accesses.data_accesses.insert(access.clone(), 1);

    let access_diff = AccessDiff {
        new_accesses: Default::default(),
        removed_accesses: vec![access].into_iter().collect(),
    };
    data_accesses.apply_access_diff(access_diff.clone());
    data_accesses.apply_access_diff(access_diff);

    assert!(data_accesses.get_data_accesses(Chain::default()).is_empty());
}
