use foundry_evm::backend::Access;
use std::collections::HashSet;

/// A diff between the last two sets of accesses
#[derive(Clone, Default, Debug)]
pub struct AccessDiff {
    /// The new accesses that were added since the last set of accesses
    pub new_accesses: HashSet<Access>,
    /// The accesses from the last set that were no longer needed
    pub removed_accesses: HashSet<Access>,
}

impl AccessDiff {
    /// Create a new AccessDiff from two sets of accesses
    pub fn new(recorded_accesses: Vec<Access>, previous_accesses: Vec<Access>) -> Self {
        let mut previous_accesses = previous_accesses;

        let mut new_accesses = HashSet::new();

        for access in recorded_accesses.into_iter() {
            // If the access is in the previous accesses, remove it from the previous accesses as
            // the remaining previous accesses become the removed accesses
            if let Some(pos) = previous_accesses.iter().position(|a| *a == access) {
                previous_accesses.remove(pos);
            }
            // If the access is not in the previous accesses, add it to the new accesses
            else {
                new_accesses.insert(access);
            }
        }

        AccessDiff { new_accesses, removed_accesses: HashSet::from_iter(previous_accesses) }
    }
}

#[test]
fn test_access_diffs_count() {
    use crate::test::get_access;
    let previous_accesses = vec![get_access()];
    let recorded_accesses = vec![Access { chain: 2.into(), ..get_access() }];

    let access_diff = AccessDiff::new(recorded_accesses.clone(), previous_accesses.clone());

    assert_eq!(access_diff.new_accesses.len(), 1);
    assert_eq!(access_diff.removed_accesses.len(), 1);

    assert_eq!(access_diff.new_accesses, HashSet::from_iter(recorded_accesses));

    assert_eq!(access_diff.removed_accesses, HashSet::from_iter(previous_accesses));
}

#[test]
fn test_access_diffs_no_count() {
    use crate::test::get_access;
    let previous_accesses = vec![get_access()];
    let recorded_accesses = vec![get_access()];

    let access_diff = AccessDiff::new(recorded_accesses, previous_accesses);

    assert_eq!(access_diff.new_accesses.len(), 0);
    assert_eq!(access_diff.removed_accesses.len(), 0);

    assert_eq!(access_diff.new_accesses, HashSet::new());

    assert_eq!(access_diff.removed_accesses, HashSet::new());
}
