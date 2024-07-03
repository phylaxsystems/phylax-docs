use foundry_cli::opts::{CoreBuildArgs, ProjectPathsArgs};
use opentelemetry::Context as OtelContext;
use phylax_interfaces::{
    activity::Activity,
    context::ActivityContext,
    event::EventBus,
    executors::TaskManager,
    state_registry::{StateRegistry, StateRegistryBuilder},
};
use phylax_runner::{Backend, DataAccesses};
use phylax_tasks::{
    monitors::build_monitor_channels,
    task::{ActivityCategory, TaskDetails},
};

pub const RPC_URL: &str = "https://eth-mainnet.g.alchemy.com/v2/jPALvRRJ_DwXYhaRiZ2cVvEU0SVkkiPj";

pub const ALERTS_BASE_PATH: &str = "../../testdata/alerts";

fn get_build_args(base_path: &str) -> CoreBuildArgs {
    let base_path = base_path.to_owned();

    CoreBuildArgs {
        project_paths: ProjectPathsArgs {
            root: Some(base_path.clone().into()),
            config_path: Some((base_path.clone() + "/foundry.toml").into()),
            contracts: Some(("./").into()),

            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn get_alerts_build_args() -> CoreBuildArgs {
    get_build_args(ALERTS_BASE_PATH)
}

pub fn get_state_registry() -> StateRegistry {
    let mut builder = StateRegistryBuilder::default();
    builder.insert(Backend::spawn(None));
    builder.insert(DataAccesses::default());
    builder.build()
}

pub fn get_activity_context<A>(category: ActivityCategory) -> ActivityContext
where
    A: Activity,
{
    let task_details = TaskDetails {
        task_name: "task_name".to_string(),
        task_flavor: "task_flavor".to_string(),
        task_labels: &[],
        task_category: category,
    };
    let task_otel_context = OtelContext::new();
    let monitors = build_monitor_channels(A::MONITORS, &task_details, &task_otel_context);

    ActivityContext {
        state_registry: get_state_registry().into(),
        activity_monitors: monitors.activity_senders,
        executor: TaskManager::current().executor(),
        health_monitors: monitors.health_senders,
        event_bus: EventBus::new(),
    }
}

pub fn get_alert_context<A>() -> ActivityContext
where
    A: Activity,
{
    get_activity_context::<A>(ActivityCategory::Alert)
}

pub fn get_watcher_context<A>() -> ActivityContext
where
    A: Activity,
{
    get_activity_context::<A>(ActivityCategory::Watcher)
}

pub mod macros;
