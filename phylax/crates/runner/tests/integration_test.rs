use forge::backend::Backend;
use forge_cmd::FilterArgs;
use phylax_runner::{AssertionOutcome, Compile, DataAccesses, PhylaxRunner};
use phylax_test_utils::get_alerts_build_args;

use regex::Regex;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_storage_does_not_persist_across_runs() {
    let filter = FilterArgs {
        contract_pattern: Some(Regex::new("StorageAlert").unwrap()),
        ..Default::default()
    };

    let runner = PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap();

    let db = Backend::spawn(None);

    let failures_a = runner.execute_assertions(db.clone()).0.failures;
    let failures_b = runner.execute_assertions(db).0.failures;

    assert!(failures_a.is_empty(), "Unexpected failures, run a {failures_a:#?}");
    assert!(failures_b.is_empty(), "Unexpected failures, run b {failures_b:#?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_fork_switch() {
    let filter = FilterArgs {
        contract_pattern: Some(Regex::new("ForkAlert").unwrap()),
        ..Default::default()
    };

    let runner = PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap();

    let db = Backend::spawn(None);

    let failures = runner.execute_assertions(db).0.failures;
    assert!(failures.is_empty(), "Unexpected failures {failures:#?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simple_alert() {
    let filter = FilterArgs {
        contract_pattern: Some(Regex::new("SimpleAlert").unwrap()),
        ..Default::default()
    };

    let runner = PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap();

    let db = Backend::spawn(None);

    let (AssertionOutcome { exports, failures }, _) = runner.execute_assertions(db);

    assert_eq!(exports.get("chainid"), Some(&"1".to_owned()), "unexpected chainid");

    assert_eq!(exports.get("block.number"), Some(&"420".to_owned()), "Unexpected block.number");

    assert!(failures.is_empty(), "Unexpected failures {failures:#?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_same_fork_same_backend() {
    let filter = FilterArgs {
        contract_pattern: Some(Regex::new("SimpleAlert").unwrap()),
        ..Default::default()
    };
    let runner = PhylaxRunner::new(get_alerts_build_args().compile().unwrap(), filter).unwrap();
    let fork_url = phylax_test_utils::RPC_URL;

    let db = Backend::spawn(None);

    let now = std::time::Instant::now();
    let (AssertionOutcome { failures, .. }, access_diff) = runner.execute_assertions(db.clone());
    assert!(failures.is_empty(), "Unexpected failures {failures:#?}");
    println!("Time taken run 0a: {:?}", now.elapsed());
    println!("Access diff 0a: {:#?}", access_diff);

    let now = std::time::Instant::now();
    let (AssertionOutcome { failures, .. }, access_diff_b) = runner.execute_assertions(db);
    assert!(failures.is_empty(), "Unexpected failures {failures:#?}");
    println!("Time taken run 0b: {:?}", now.elapsed());
    println!("Access diff 0b: {:#?}", access_diff_b);

    let data_accesses = DataAccesses::default();

    let now = std::time::Instant::now();
    data_accesses.apply_access_diff(access_diff);
    println!("Time taken apply access diff: {:?}", now.elapsed());
    println!("Accesses: {:#?}", data_accesses);

    let db = Backend::spawn(None);
    let now = std::time::Instant::now();
    let run_1_total = std::time::Instant::now();
    db.load_accesses(
        &data_accesses.get_data_accesses(1.into()),
        1.into(),
        420,
        fork_url.to_string(),
    )
    .unwrap();
    println!("Time taken load accesses run 1: {:?}", now.elapsed());

    let now = std::time::Instant::now();
    assert!(runner.execute_assertions(db).0.failures.is_empty());

    println!("Time taken run 1 assertions: {:?}", now.elapsed());

    println!("Time taken run 1 total: {:?}", run_1_total.elapsed());

    //Try with fresh Backend
    let db = Backend::spawn(None);
    let now = std::time::Instant::now();
    assert!(runner.execute_assertions(db).0.failures.is_empty());
    println!("Time taken run 2: {:?}", now.elapsed());
}
