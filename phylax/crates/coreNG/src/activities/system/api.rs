use async_trait::async_trait;
use flume::r#async::RecvStream;
use phylax_api::{
    Api, ApiInternalCmd,
    ApiTaskStatus::{Running, Stopped},
    SystemEvent,
};
use phylax_tracing::tracing::{debug, info};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};
use tokio::task::JoinHandle;

use super::orchestrator::{InternalTaskStatus, TaskStatusEventBody};

use crate::{
    activities::{Activity, ActivityContext, FgWorkType},
    events::{Event, EventBuilder, EventType},
    tasks::task::{ActivityState, WorkContext},
};
use eyre::Result;

/// The `Api` struct implements the `Activity` trait.
#[async_trait]
impl Activity for Api {
    /// The type of command that this activity uses.
    type Command = ApiCommand;
    /// The type of the internal stream that this activity uses.
    type InternalStream = RecvStream<'static, ApiInternalCmd>;
    /// The type of the items in the internal stream.
    type InternalStreamItem = ApiInternalCmd;

    /// Returns the work type of the `Api` activity.
    /// This is always `FgWorkType::NonBlocking`.
    fn work_type(&self) -> FgWorkType {
        FgWorkType::NonBlocking
    }

    /// Cleans up the `Api` activity.
    /// Currently, this function does nothing and always returns `Ok`.
    async fn cleanup(self) -> Result<()> {
        Ok(())
    }

    /// Returns the internal stream of the `Api` activity. The internal stream is used
    /// by the API to emit events based on requests it receives at it's endpoints
    async fn internal_stream(&self) -> Option<Self::InternalStream> {
        Some(self.state.internal_bus.cmd_rx.clone().into_stream())
    }

    /// Decides what commands to execute based on the given events.
    /// It expects the following event types:
    /// - TaskStatusChange: Emitted by the [`super::orchestrator::Orchestrator`] when the
    /// [`sisyphus_tasks::sisyphus::TaskStatus`] of one of the tasks changes
    /// - ActivityStateChange: Emitted by every task when it starts/stops background/foreground work
    /// It also processes every event so that it can be exposed via the API for consumption
    fn decide(&self, events: Vec<&Event>, ctx: &WorkContext) -> Option<Vec<ApiCommand>> {
        let cmds = events
            .into_iter()
            .map(|event| match event.get_type() {
                EventType::TaskStatusChange => {
                    let body: TaskStatusEventBody =
                        serde_json::from_value(event.get_body_ref().to_owned()).unwrap();
                    ApiCommand::SetState {
                        name: body.task,
                        status: body.state,
                        event: event.clone(),
                    }
                }
                EventType::ActivityStateChange => {
                    let body: ActivityState =
                        serde_json::from_value(event.get_body_ref().to_owned()).unwrap();
                    let name = ctx
                        .activity
                        .dns
                        .find_name_from_id(event.get_origin())
                        .unwrap_or("unknown task".to_owned());
                    ApiCommand::SetWorking {
                        name,
                        working_bg: body.background_working,
                        working_fg: body.foreground_working,
                        event: event.clone(),
                    }
                }
                _ => ApiCommand::PublishEvent { event: event.clone() },
            })
            .collect::<Vec<ApiCommand>>();
        Some(cmds)
    }

    /// Executes the given commands.
    async fn do_work(&mut self, cmds: Vec<ApiCommand>, _ctx: WorkContext) -> Result<()> {
        for cmd in cmds {
            match cmd {
                ApiCommand::SetWorking { name, working_bg, working_fg, event } => {
                    self.state.internal_bus.send_event(event)?;
                    self.set_task_working(name, working_bg, working_fg)?;
                }
                ApiCommand::SetState { name, status, event } => {
                    self.state.internal_bus.send_event(event)?;
                    self.set_task_state(name, status)?;
                }
                ApiCommand::PublishEvent { event } => {
                    self.state.internal_bus.send_event(event)?;
                }
            }
        }
        Ok(())
    }

    /// Handles work from the internal stream.
    /// The internal stream currently can be one of the following:
    /// - [`ApiInternalCmd::ActivateTask`]: Sent by the API when the `tasks/{task}` endpoint is
    ///   invoked. Used to activate tasks programmatically.
    async fn internal_stream_work(
        &mut self,
        item: Self::InternalStreamItem,
        _ctx: Arc<ActivityContext>,
    ) -> Option<Event> {
        match item {
            ApiInternalCmd::ActivateTask(task) => {
                info!(task, "[API] received a 'Task Activation' request");
                let event = EventBuilder::new()
                    .with_type(EventType::ActivateTask)
                    .with_body(serde_json::to_value(ApiEventBody::ActivateTask(task)).unwrap())
                    .build();
                debug!(event=?event, "Event - [Api -> Event Bus]");
                Some(event)
            }
        }
    }

    /// Spawns a background task to run the server and returns a handle to the task.
    fn spawn_bg_work(&mut self) -> Option<JoinHandle<Result<()>>> {
        let api_config = self.prepare_server();
        let res = tokio::task::spawn(async move {
            Api::run_prepared_server(api_config).await?;
            Ok(())
        });
        Some(res)
    }
}

/// The commands that the `Api` activity can execute.
#[derive(PartialEq)]
pub enum ApiCommand {
    /// Sets the working state of a task.
    SetWorking { name: String, working_fg: bool, working_bg: bool, event: Event },
    /// Sets the state of a task.
    SetState { name: String, status: InternalTaskStatus, event: Event },
    /// Publishes an event.
    PublishEvent { event: Event },
}
/// Debug implementation for `ApiCommand`.
impl Debug for ApiCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiCommand::SetWorking { name, working_bg, working_fg, event: _ } => {
                write!(f, "SetWorking: {name} | background: {working_bg}, foreground: {working_fg}",)
            }
            ApiCommand::SetState { name, status, event: _ } => {
                write!(f, "SetState: {name} | {:?}", status)
            }
            ApiCommand::PublishEvent { event } => {
                write!(f, "PublishEvent | {event}")
            }
        }
    }
}

/// The body of an event that the `Api` activity can generate.
#[derive(Serialize, Deserialize)]
pub enum ApiEventBody {
    /// Activates a task.
    ActivateTask(String),
}

/// Converts an `InternalTaskStatus` to an `ApiTaskStatus`.
impl From<InternalTaskStatus> for phylax_api::ApiTaskStatus {
    fn from(status: InternalTaskStatus) -> Self {
        match status {
            InternalTaskStatus::Running => Running { working_fg: false, working_bg: false },
            InternalTaskStatus::Starting => Running { working_fg: false, working_bg: false },
            InternalTaskStatus::Stopped(_exceptional, reason) => Stopped { reason },
            InternalTaskStatus::Panicked => Stopped { reason: vec!["Panicked".to_owned()] },
            InternalTaskStatus::Recovering(reason) => {
                phylax_api::ApiTaskStatus::Restarting { reason }
            }
        }
    }
}

/// Converts an `Event` to a `SystemEvent`.
impl From<Event> for SystemEvent {
    fn from(event: Event) -> Self {
        Self {
            origin: event.get_origin().to_string(),
            event_type: event.get_type().to_string(),
            timestamp: event.get_timestamp(),
            body: event.get_body(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::mock_context::mock_activity_context;
    use phylax_common::metrics::MetricsRegistry;
    use phylax_config::PhConfig;
    use reqwest::StatusCode;
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::Arc,
    };
    use tokio_stream::StreamExt;

    fn setup() -> Api {
        let bind_ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let metrics_registry = MetricsRegistry::new();
        let config: Arc<PhConfig> = Arc::new(PhConfig::default());
        Api::new(bind_ip, metrics_registry, config)
    }

    #[test]
    fn test_work_type() {
        let api = setup();
        assert_eq!(api.work_type(), FgWorkType::NonBlocking);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let api = setup();
        assert!(api.cleanup().await.is_ok());
    }
    #[test]
    fn test_system_event_from_event() {
        let event = EventBuilder::new().build();
        let system_event = SystemEvent::from(event.clone());
        assert_eq!(system_event.origin, event.get_origin().to_string());
        assert_eq!(system_event.event_type, event.get_type().to_string());
        assert_eq!(system_event.timestamp, event.get_timestamp());
        assert_eq!(system_event.body, event.get_body());
    }

    #[tokio::test]
    async fn test_do_work_set_working() {
        let mut api = setup();
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let event = EventBuilder::new().build(); // Create a valid event here
        let command = ApiCommand::SetWorking {
            name: "test".to_string(),
            working_bg: true,
            working_fg: false,
            event: event.clone(),
        };
        assert!(api.do_work(vec![command], context).await.is_ok());
        let task_state = api.state.tasks_state.get("test").unwrap();
        assert!(task_state.is_running());
        assert_eq!(Some(true), task_state.working_bg());
        assert_eq!(Some(false), task_state.working_fg());

        let system_event = api.state.internal_bus.async_recv_event().await.unwrap();
        assert_eq!(SystemEvent::from(event), system_event);
    }

    #[tokio::test]
    async fn test_do_work_set_state() {
        let mut api = setup();
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let event = EventBuilder::new().with_origin(234).build(); // Create a valid event here
        let error_chain = vec!["first error".to_owned(), "second error".to_owned()];
        let command = ApiCommand::SetState {
            name: "test".to_string(),
            status: InternalTaskStatus::Stopped(true, error_chain.clone()),
            event: event.clone(),
        };
        assert!(api.do_work(vec![command], context).await.is_ok());
        let task_state = api.state.tasks_state.get("test").unwrap();
        assert!(task_state.is_stopped());
        assert_eq!(task_state.stopped_reason(), Some(&error_chain));

        let system_event = api.state.internal_bus.async_recv_event().await.unwrap();
        assert_eq!(SystemEvent::from(event), system_event);
    }

    #[tokio::test]
    async fn test_do_work_publish_event() {
        let mut api = setup();
        let context = WorkContext::new(mock_activity_context(None), EventBuilder::new().build());
        let event = EventBuilder::new().with_type(EventType::AlertFiring).build(); // Create a valid event here
        let command = ApiCommand::PublishEvent { event: event.clone() };
        assert!(api.do_work(vec![command], context.clone()).await.is_ok());
        let system_event = api.state.internal_bus.async_recv_event().await.unwrap();
        assert_eq!(SystemEvent::from(event), system_event);
    }

    #[test]
    fn test_internal_task_status_conversion() {
        let internal_status = InternalTaskStatus::Running;
        let api_status = phylax_api::ApiTaskStatus::from(internal_status);
        assert_eq!(api_status, Running { working_fg: false, working_bg: false });

        let internal_status = InternalTaskStatus::Starting;
        let api_status = phylax_api::ApiTaskStatus::from(internal_status);
        assert_eq!(api_status, Running { working_fg: false, working_bg: false });

        let internal_status = InternalTaskStatus::Stopped(true, vec!["error".to_owned()]);
        let api_status = phylax_api::ApiTaskStatus::from(internal_status);
        assert_eq!(api_status, Stopped { reason: vec!["error".to_owned()] });

        let internal_status = InternalTaskStatus::Panicked;
        let api_status = phylax_api::ApiTaskStatus::from(internal_status);
        assert_eq!(api_status, Stopped { reason: vec!["Panicked".to_owned()] });

        let internal_status = InternalTaskStatus::Recovering("recovering".to_owned());
        let api_status = phylax_api::ApiTaskStatus::from(internal_status);
        assert_eq!(
            api_status,
            phylax_api::ApiTaskStatus::Restarting { reason: "recovering".to_owned() }
        );
    }

    #[tokio::test]
    async fn test_internal_stream() {
        let api = setup();
        let stream = api.internal_stream().await;
        assert!(stream.is_some());

        let cmd = ApiInternalCmd::ActivateTask("KillMaker".to_owned());
        api.state.internal_bus.send_cmd(cmd.clone()).unwrap();

        let item = stream.unwrap().next().await.unwrap();
        assert_eq!(cmd, item);
    }

    #[tokio::test]
    async fn test_internal_stream_work() {
        let mut api = setup();
        let ctx = mock_activity_context(None);
        let item = ApiInternalCmd::ActivateTask("test".to_string());
        let event = api.internal_stream_work(item, ctx.clone()).await;
        assert!(event.is_some());
        assert_eq!(event.unwrap().get_type(), &EventType::ActivateTask);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_bg_spawns_api_server() {
        let mut api = setup();
        let addr = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap()
        };
        api.bind_ip = addr;
        api.spawn_bg_work().unwrap();
        let client = reqwest::Client::new();
        let res = client.get(format!("http://{addr}")).send().await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
