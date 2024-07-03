use alloy_sol_macro::sol;
use alloy_sol_types::SolEvent;

use forge::result::{SuiteTestResult, TestOutcome, TestStatus};
use std::collections::HashMap;

sol! {
    /// Phylax Export Event, used to export values from the assertions.
    #[derive(Debug)]
    event PhylaxExport(string key, string value);
}

/// The outcome of running assertions assertions.
#[derive(Debug)]
pub struct AssertionOutcome {
    /// The failures that occurred during the assertions(if any).
    pub failures: Vec<SuiteTestResult>,
    /// The PhylaxExport event Key/Value pairs from the assertions(if any).
    pub exports: HashMap<String, String>,
}

impl From<TestOutcome> for AssertionOutcome {
    /// Creates a [`AssertionOutcome`] from a [`TestOutcome`], collecting the [`PhylaxExport`]
    /// events.
    fn from(outcome: TestOutcome) -> Self {
        // Collect the exports from the tests.
        let exports = outcome.tests().fold(HashMap::new(), |mut map, t| {
            t.1.logs
                .iter()
                .filter(|log| {
                    let topics = log.topics();
                    matches!(topics[0], PhylaxExport::SIGNATURE_HASH) && topics.len() == 1
                })
                .for_each(|log| {
                    let _ = PhylaxExport::decode_log_data(log, true)
                        .map(|PhylaxExport { key, value }| map.insert(key, value));
                });
            map
        });

        let failures = outcome
            .into_tests()
            .filter(|t| t.result.status == TestStatus::Failure)
            .collect::<Vec<_>>();

        AssertionOutcome { exports, failures }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge::result::{SuiteResult, TestOutcome, TestResult, TestStatus};
    use std::collections::BTreeMap;

    fn test_result_to_test_outcome(result: TestResult) -> TestOutcome {
        let mut test_results = BTreeMap::new();
        test_results.insert("test".to_string(), result);

        let suite_result = SuiteResult::new(Default::default(), test_results, Vec::new());

        let mut suite_results = BTreeMap::new();
        suite_results.insert("test".to_string(), suite_result);

        TestOutcome::new(suite_results, false)
    }

    #[test]
    fn assertion_outcome_empty() {
        let AssertionOutcome { failures, exports } =
            AssertionOutcome::from(TestOutcome::empty(false));

        assert_eq!(failures.len(), 0);
        assert_eq!(exports.len(), 0);
    }

    #[test]
    fn assertion_outcome_with_exports() {
        use alloy_primitives::Log;
        use forge::result::{TestResult, TestStatus};

        let key = "key".to_owned();
        let value = "value".to_owned();

        let result = TestResult {
            status: TestStatus::Success,
            logs: vec![Log::new(
                Default::default(),
                vec![PhylaxExport::SIGNATURE_HASH],
                PhylaxExport { key: key.to_owned(), value: value.to_owned() }.encode_data().into(),
            )
            .unwrap()],
            ..Default::default()
        };

        let AssertionOutcome { exports, .. } = test_result_to_test_outcome(result).into();

        assert_eq!(exports.len(), 1);
        assert_eq!(exports.get(&key), Some(&value));
    }

    #[test]
    fn assertion_outcome_with_success() {
        let result = TestResult { status: TestStatus::Success, ..Default::default() };

        let AssertionOutcome { failures, .. } = test_result_to_test_outcome(result).into();

        assert_eq!(failures.len(), 0);
    }
    #[test]
    fn assertion_outcome_with_failure() {
        let result = TestResult { status: TestStatus::Failure, ..Default::default() };

        let AssertionOutcome { failures, .. } = test_result_to_test_outcome(result).into();

        assert_eq!(failures.len(), 1);
    }
}
