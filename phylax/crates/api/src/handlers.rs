use crate::{internal_bus::ApiInternalCmd, ApiState, EyreResult};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
};
use axum_extra::response::ErasedJson;

use serde::Deserialize;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};
use utoipa::IntoParams;

#[utoipa::path(
        get,
        path = "/",
        responses(
            (status = 200, description = "Welcome to the Phylax API", body = String)
        )
    )]
pub async fn welcome() -> EyreResult<String> {
    Ok("Welcome to the API".to_owned())
}

#[utoipa::path(
        get,
        path = "/metrics",
        responses(
            (status = 200, description = "Prometheus Metrics Endpoint", body = String)
        )
    )]
pub async fn metrics(State(state): State<Arc<ApiState>>) -> EyreResult<String> {
    let metrics = state.metrics_registry.gather_text()?;
    let res = String::from_utf8(metrics)?;
    Ok(res)
}

#[utoipa::path(
        get,
        path = "/api/v1/tasks/{task_name}",
        responses(
            (status = 200, description = "Get the status of a task", body = ApiTaskStatus),
            (status = 404, description = "The task name does not exist")
        ),
        security(
        ("JWT" = [])
        )
    )]
pub async fn get_status(
    State(state): State<Arc<ApiState>>,
    Path(task_name): Path<String>,
) -> Result<ErasedJson, axum::http::StatusCode> {
    let task = state.tasks_state.get(&task_name);
    match task {
        Some(task_state) => Ok(ErasedJson::pretty(task_state)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
        get,
        path = "/api/v1/tasks",
        responses(
            (status = 200, description = "Get the status of a task", body = TaskStates),
        ),
        security(
        ("JWT" = [])
        )

    )]
pub async fn get_status_all(State(state): State<Arc<ApiState>>) -> ErasedJson {
    ErasedJson::pretty(&state.tasks_state)
}

#[utoipa::path(
        get,
        path = "/api/v1/orchestrator/config",
        responses(
            (status = 200, description = "Get running config of Phylax", body = PhConfig),
        ),
        security(
        ("JWT" = [])
        )
    )]
// FIXME(odysseas): schema is not rendered properly
pub async fn get_config(State(state): State<Arc<ApiState>>) -> ErasedJson {
    ErasedJson::pretty(state.config.clone())
}
#[utoipa::path(
        post,
        path = "/api/v1/tasks/{task_name}",
        responses(
            (status = 200, description = "An ApiCommand event will be emitted with the name of the task. If the task is subscribed to this, it will be notified.", body = String),
            (status = 500, description = "The API could not sent the event in the event bus"),
        ),
        security(
        ("JWT" = [])
        )
    )]
// FIXME(odysseas): schema is not rendered properly
pub async fn activate_task(
    State(state): State<Arc<ApiState>>,
    Path(action): Path<String>,
) -> Result<String, impl IntoResponse> {
    if !state.tasks_state.contains_key(&action) {
        Err((StatusCode::NOT_FOUND, "Task not found"))
    } else {
        state
            .internal_bus
            .send_cmd(ApiInternalCmd::ActivateTask(action.clone()))
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to send command"))?;
        Ok(format!("Task {} is activated", action))
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct EventFilters {
    from_origin: Option<String>,
    of_event_type: Option<String>,
}
#[utoipa::path(
        get,
        path = "/api/v1/events",
        params(EventFilters),
        responses(
            (status = 200, description = "SSE endpoint that streams all internal events. This request will never return (SSE) and can't be used from inside RapiDoc's UI. Use curl.", body = EyreResult<String>),
            (status = 500, description = "The server encountered some error. Check the logs.", body = EyreResult<String>),
        ),
        security(
        ("JWT" = [])
        )
    )]
pub async fn get_events(
    State(state): State<Arc<ApiState>>,
    Query(filters): Query<EventFilters>,
) -> Sse<impl Stream<Item = EyreResult<Event>>> {
    let rx = state.internal_bus.events_rx.clone();
    let stream = rx
        .into_stream()
        .filter(move |event| {
            let event_type = filters
                .of_event_type
                .as_ref()
                .map(|event_type| &event.event_type == event_type)
                .unwrap_or(true);
            let origin =
                filters.from_origin.as_ref().map(|origin| &event.origin == origin).unwrap_or(true);
            event_type && origin
        })
        .map(|event| {
            let event_type = event.event_type.clone();
            let event = Event::default().json_data(event)?.event(event_type);
            Ok(event)
        });
    Sse::new(stream)
}

pub async fn handler_404(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let ip = state.config.api.bind_ip;
    let return_message = if state.config.api.enable_api {
        format!("<p>Incorrect API route</p>\n<p>If API is enabled, visit 'http://{ip}/docs' for the API docs</p>")
    } else {
        format!("<p>API is disabled</p>\n<p>Only accessible route is: 'http://{ip}/metrics</p>")
    };
    (StatusCode::NOT_FOUND, return_message)
}
