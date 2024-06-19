use super::handlers::*;
use crate::{ApiTaskStatus, TaskStates};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
        modifiers(&SecurityAddon),
        paths(
            welcome,
            metrics,
            get_status,
            get_status_all,
            get_config,
            activate_task,
            get_events
        ),
        components(
            schemas(ApiTaskStatus, TaskStates)
        ),
        tags(
            (name = "phylax", description = "Blockchain Alerting and Incident Response")
        ),
        external_docs(url = "http://github.com/phylax-systems/phylax", description = "Phylax repository"),
        external_docs(url = "https://graph.phylax.watch", description = "The Phylax Graph (Docs, Blog, etc)"),
        servers(
         (url = "http://{server_ip}:{port}", description = "Local Phylax instance",
         variables(
            ("port" = (default = "4269", description = "Port")),
            ("server_ip" = (default = "127.0.0.1", description = "Phylax IP"))
            ))
        )
)]
pub struct ApiDoc;
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.components = Some(
            utoipa::openapi::ComponentsBuilder::new()
                .security_scheme(
                    "JWT",
                    SecurityScheme::Http(
                        HttpBuilder::new()
                            .scheme(HttpAuthScheme::Bearer)
                            .bearer_format("JWT")
                            .build(),
                    ),
                )
                .build(),
        )
    }
}
