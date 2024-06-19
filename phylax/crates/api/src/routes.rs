use crate::{docs::ApiDoc, handlers::*, ApiState};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;

/// This function sets up the routes for the API server.
pub fn protected_routes() -> Router<Arc<ApiState>> {
    Router::new().nest(
        "/api/v1/",
        Router::new()
            .route("/tasks", get(get_status_all))
            .route("/tasks/", get(get_status_all))
            // FIXME: /tasks/ should redirect to /tasks
            .route("/tasks/:task_name", get(get_status))
            .route("/tasks/:task_name", post(activate_task))
            .route("/events", get(get_events))
            .route("/orchestrator/config", get(get_config)),
    )
}

pub fn unprotected_routes() -> Router<Arc<ApiState>> {
    let api_doc = ApiDoc::openapi();
    Router::new()
        .route("/", get(welcome))
        .fallback(handler_404)
        .merge(RapiDoc::with_openapi("/docs/open-api.json", api_doc).path("/docs"))
}

pub fn metrics_routes() -> Router<Arc<ApiState>> {
    Router::new().route("/metrics", get(metrics))
}
