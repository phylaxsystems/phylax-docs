// use crate::{
//     events::{EventBuilder, EventBus},
//     tasks::{
//         dns::TaskDns,
//         spawn::TaskBuilder,
//         subscriptions::{Subscription, SubscriptionCounter, Subscriptions},
//         task::TaskCategory,
//     },
//     tests::mocks::activity_mock::{MockActivity, MockActivityStatus, NonBlockingMockActivity},
// };
// use phylax_common::metrics::MetricsRegistry;

// use std::{sync::Arc, thread::sleep, time::Duration};
// use tokio::sync::watch;
// use tracing::{info_span, Span};

// #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// async fn test_task_bootstrap() {
//     // Create a new MockActivity
//     let (tx, rx) = watch::channel(MockActivityStatus::default());
//     let name = "test_task";
//     let subs = Subscriptions::default();
//     let dns = Arc::new(TaskDns::default());
//     let metrics = MetricsRegistry::default();
//     let bus = EventBus::default();

//     let artifacts =
//         NonBlockingMockActivity::build_and_spawn(name, subs, dns.clone(), metrics, bus, tx);
//     let id = dns.get_id_from_name(name);
//     assert_eq!(artifacts.id, id);
//     sleep(Duration::from_millis(1000));
//     let status = rx.borrow();
//     assert!(status.bootstraped);
// }

// #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// async fn test_task_read_events() {
//     // Create a new MockActivity
//     let (tx, rx) = watch::channel(MockActivityStatus::default());
//     let name = "test_task";
//     let rule = "true".to_owned();
//     let sub = Subscription::new(rule, SubscriptionCounter::default());
//     let mut subs = Subscriptions::default();
//     subs.add(sub);
//     let dns = Arc::new(TaskDns::default());
//     let metrics = MetricsRegistry::default();
//     let mut bus = EventBus::default();

//     let artifacts = NonBlockingMockActivity::build_and_spawn(
//         name,
//         subs,
//         dns.clone(),
//         metrics.clone(),
//         bus.get_copy(),
//         tx,
//     );
//     let id = dns.get_id_from_name(name);
//     assert_eq!(artifacts.id, id);
//     sleep(Duration::from_millis(1000));
//     let status = rx.borrow();
//     assert!(status.bootstraped);

//     let event = EventBuilder::new().with_origin(99).build();
//     bus.publish(event.clone());

//     let status = rx.borrow();
//     assert_eq!(status.last_received_event, event);
// }
