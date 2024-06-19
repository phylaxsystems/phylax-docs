use crate::{
    auth::{auth_middleware, jwt_decoder},
    internal_bus::ApiInternalBus,
    routes::{metrics_routes, protected_routes, unprotected_routes},
    task_state::ApiTaskStatus,
    task_states::TaskStates,
    ApiError,
};
use axum::{middleware, Router};
use axum_jwt_auth::Decoder;
use phylax_common::metrics::MetricsRegistry;
use phylax_config::PhConfig;
use phylax_tracing::tracing::{debug, info};
use std::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
    sync::Arc,
};
use tokio::task::JoinHandle;
use tower_http::trace::TraceLayer;

pub struct Api {
    pub bind_ip: SocketAddr,
    pub state: Arc<ApiState>,
    pub server_handle: Option<JoinHandle<Result<(), ApiError>>>,
}

pub struct ApiState {
    pub config: Arc<PhConfig>,
    pub metrics_registry: MetricsRegistry,
    pub tasks_state: TaskStates,
    pub internal_bus: ApiInternalBus,
    pub jwt_decoder: Option<Decoder>,
}

impl ApiState {
    pub fn auth_enabled(&self) -> bool {
        self.jwt_decoder.is_some()
    }
}

impl Api {
    pub fn new(
        bind_ip: SocketAddr,
        metrics_registry: MetricsRegistry,
        config: Arc<PhConfig>,
    ) -> Self {
        let internal_bus = ApiInternalBus::new();
        let tasks_state = TaskStates::new();
        let jwt_decoder = config.api.jwt_secret.as_ref().map(|key| jwt_decoder(key.expose()));
        let state = ApiState { metrics_registry, config, internal_bus, tasks_state, jwt_decoder };
        Self { state: Arc::new(state), bind_ip, server_handle: None }
    }
    pub fn set_task_state(
        &self,
        task_name: String,
        state: impl Into<ApiTaskStatus>,
    ) -> Result<(), ApiError> {
        self.state.tasks_state.insert(task_name, state.into());
        Ok(())
    }

    pub fn set_task_working(
        &self,
        task_name: String,
        working_bg: bool,
        working_fg: bool,
    ) -> Result<(), ApiError> {
        self.state.tasks_state.insert(task_name, ApiTaskStatus::Running { working_bg, working_fg });
        Ok(())
    }

    pub fn prepare_server(&mut self) -> ServerConfig {
        info!(bind_ip=%self.bind_ip, "Preparing API server");
        let bind_ip = self.bind_ip;
        let state = self.state.clone();
        ServerConfig { bind_ip, state }
    }

    /// This function runs the prepared server.
    pub async fn run_prepared_server(config: ServerConfig) -> Result<(), ApiError> {
        let state = config.state.clone();
        let mut app = Router::new().merge(unprotected_routes());
        if config.state.config.api.enable_api {
            debug!("Api endpoints Enabled");
            let mut protected = protected_routes();
            if state.auth_enabled() {
                debug!("Authentication Enabled");
                protected = protected
                    .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));
            }
            app = app.merge(protected)
        }
        if config.state.config.api.enable_metrics {
            debug!("Metrics Enabled");
            app = app.merge(metrics_routes());
        }
        app = app.layer(TraceLayer::new_for_http());
        let app = app.with_state(state.clone());
        let listener = tokio::net::TcpListener::bind(&config.bind_ip).await?;
        axum::serve(listener, app).await.unwrap();
        info!("API server shut down");
        Ok(())
    }
}

pub struct ServerConfig {
    pub bind_ip: SocketAddr,
    pub state: Arc<ApiState>,
}

impl Display for Api {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "API")
    }
}

impl std::fmt::Debug for Api {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "API")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_state::ApiTaskStatus;
    use phylax_common::metrics::MetricsRegistry;
    use phylax_config::PhConfig;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn test_new() {
        let bind_ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let metrics_registry = MetricsRegistry::new();
        let config = Arc::new(PhConfig::default());

        let api = Api::new(bind_ip, metrics_registry, config);

        assert_eq!(api.bind_ip, bind_ip);
        assert!(api.server_handle.is_none());
    }

    #[tokio::test]
    async fn test_set_task_state() {
        let bind_ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let metrics_registry = MetricsRegistry::new();
        let config = Arc::new(PhConfig::default());

        let api = Api::new(bind_ip, metrics_registry, config);
        let task_name = "test_task".to_string();
        let task_state = ApiTaskStatus::Running { working_bg: true, working_fg: false };

        assert!(api.set_task_state(task_name.clone(), task_state.clone()).is_ok());
        assert_eq!(api.state.tasks_state.get(&task_name).unwrap(), task_state);
    }

    #[tokio::test]
    async fn test_set_task_working() {
        let bind_ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let metrics_registry = MetricsRegistry::new();
        let config = Arc::new(PhConfig::default());

        let api = Api::new(bind_ip, metrics_registry, config);
        let task_name = "test_task".to_string();

        assert!(api.set_task_working(task_name.clone(), true, false).is_ok());
        assert_eq!(
            api.state.tasks_state.get(&task_name).unwrap(),
            ApiTaskStatus::Running { working_bg: true, working_fg: false }
        );
    }

    #[tokio::test]
    async fn test_task_state_overwrite() {
        // Setup
        let api = setup_api();
        let task_name = "test_task".to_string();

        // Set initial task state
        assert!(api
            .set_task_state(
                task_name.clone(),
                ApiTaskStatus::Running { working_bg: true, working_fg: false }
            )
            .is_ok());
    }

    #[tokio::test]
    async fn test_task_state_retrieval() {
        // Setup
        let api = setup_api();
        let task_name = "test_task".to_string();
        let task_state = ApiTaskStatus::Running { working_bg: true, working_fg: false };

        // Set task state
        assert!(api.set_task_state(task_name.clone(), task_state.clone()).is_ok());

        // Retrieve task state
        assert_eq!(api.state.tasks_state.get(&task_name).unwrap(), task_state);
    }

    #[tokio::test]
    async fn test_server_preparation() {
        // Setup
        let mut api = setup_api();

        // Prepare server
        let server_config = api.prepare_server();

        // Check that server was prepared correctly
        assert_eq!(server_config.bind_ip, api.bind_ip);
        assert!(Arc::ptr_eq(&server_config.state, &api.state));
    }

    fn setup_api() -> Api {
        let bind_ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let metrics_registry = MetricsRegistry::new();
        let config = Arc::new(PhConfig::default());
        Api::new(bind_ip, metrics_registry, config)
    }
}
