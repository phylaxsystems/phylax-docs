use crate::{
    activities::{Activity, ActivityContext, ActivityError, FgWorkType},
    events::Event,
};
use async_trait::async_trait;
use eyre::Result;
use flume::r#async::RecvStream;
use tokio::sync::watch;

use crate::tasks::task::WorkContext;
use std::{fmt::Display, sync::Arc};
use tokio::task::JoinHandle;

#[derive(Default, Debug, Clone)]
pub(crate) enum MockCommand {
    #[default]
    FirstCommand,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct MockInternalItem {
    _work: bool,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct MockActivityStatus {
    pub bootstraped: bool,
    pub decide_invocations: u32,
    pub work_invocations: u32,
    pub last_received_events: Vec<Event>,
    pub _last_internal_item: MockInternalItem,
    pub _working: bool,
}

/// Macro to generate mock activities
macro_rules! mock_activity {
    ($name:ident, $work_type:expr) => {
        pub(crate) struct $name {
            status_tx: watch::Sender<MockActivityStatus>,
            internal_item_rx: flume::Receiver<MockInternalItem>,
        }

        /// Implementation for $name
        impl $name {
            /// Creates a new instance of $name
            ///
            /// # Returns
            ///
            /// * `Self`: A new instance of $name
            pub(crate) fn new(
            ) -> (Self, watch::Receiver<MockActivityStatus>, flume::Sender<MockInternalItem>) {
                let (status_tx, status_rx) = watch::channel(MockActivityStatus::default());
                let (internal_item_tx, internal_item_rx) = flume::unbounded();
                (Self { status_tx, internal_item_rx }, status_rx, internal_item_tx)
            }

            //     pub(crate) fn build_and_spawn(name: &str, subs: Subscriptions, dns: Arc<TaskDns>,
            // metrics: MetricsRegistry, bus: EventBus, status_tx:
            // watch::Sender<MockActivityStatus>, internal_item_rx:
            // flume::Receiver<MockInternalItem>) -> SpawnArtifact {     let mock_activity =
            // Self::new(status_tx,  internal_item_rx);     // Create a new Task with the
            // MockActivity     let id = dns.get_id_from_name(name);
            //     let builder = TaskBuilder::new()
            //         .with_name(name)
            //         .with_id(id)
            //         .with_span(info_span!(parent: Span::none(), "[Task]", name, id))
            //         .with_category(TaskCategory::System)
            //         .with_subscriptions(subs)
            //         .with_event_bus(bus)
            //         .with_metrics(metrics.clone())
            //         .with_dns(dns);
            //     let artifacts = builder.build_and_spawn(mock_activity).unwrap();
            //     artifacts
            //     }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, stringify!($name))
            }
        }

        #[async_trait]
        impl Activity for $name {
            type Command = MockCommand;
            type InternalStream = RecvStream<'static, MockInternalItem>;
            type InternalStreamItem = MockInternalItem;

            async fn bootstrap(&mut self, _context: Arc<ActivityContext>) -> Result<()> {
                self.status_tx.send_modify(|state| state.bootstraped = true);
                println!("bootstrapped");
                Ok(())
            }

            fn decide(
                &self,
                events: Vec<&Event>,
                _context: &WorkContext,
            ) -> Option<Vec<Self::Command>> {
                self.status_tx.send_modify(|state| {
                    state.decide_invocations += 1;
                    state.last_received_events =
                        events.iter().map(|event| (*event).clone()).collect();
                });
                if events.len() % 2 == 0 {
                    // Return a vector of commands where the number of commands is the same as the
                    // length of events
                    Some(vec![Self::Command::default(); events.len()])
                } else {
                    // Return None when the length of events is not divisible by 2
                    None
                }
            }

            async fn do_work(
                &mut self,
                _command: Vec<Self::Command>,
                _context: WorkContext,
            ) -> Result<()> {
                println!("do work!!");
                match $work_type {
                    FgWorkType::NonBlocking | FgWorkType::SpawnNonBlocking => {
                        self.status_tx.send_modify(|state| {
                            state.work_invocations += 1;
                        });
                        Ok(())
                    }
                    _ => Err(ActivityError::NotImplemented.into()),
                }
            }

            fn do_work_blocking(
                &mut self,
                _command: Vec<Self::Command>,
                _context: WorkContext,
            ) -> Result<()> {
                match $work_type {
                    FgWorkType::SpawnBlocking => {
                        self.status_tx.send_modify(|state| {
                            state.work_invocations += 1;
                        });
                        Ok(())
                    }
                    _ => Err(ActivityError::NotImplemented.into()),
                }
            }

            fn work_type(&self) -> FgWorkType {
                $work_type
            }

            async fn cleanup(self) -> Result<()> {
                Ok(())
            }

            async fn internal_stream(&self) -> Option<Self::InternalStream> {
                Some(self.internal_item_rx.clone().into_stream())
            }

            async fn internal_stream_work(
                &mut self,
                _item: Self::InternalStreamItem,
                _ctx: Arc<ActivityContext>,
            ) -> Option<Event> {
                None
            }

            fn spawn_bg_work(&mut self) -> Option<JoinHandle<Result<()>>> {
                None
            }
        }
    };
}
mock_activity! {NonBlockingMockActivity, FgWorkType::NonBlocking}
mock_activity! {SpawnNonBlockingMockActivity, FgWorkType::SpawnNonBlocking}
