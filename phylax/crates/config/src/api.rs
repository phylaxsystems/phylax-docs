use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::{substitute_env::deserialize_with_env, SensitiveValue, Trigger};

/// The configuration of the API
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ApiConfig {
    /// The IP that the API should bind to.
    /// - `0.0.0.0:4242` will accept requests from any IP, at port 4242
    /// - `127.0.0.1:6969` will accept requests only from localhost, at port 6969
    #[serde(deserialize_with = "deserialize_with_env")]
    pub bind_ip: SocketAddr,
    /// If the API task should be spawned with the API routes enabled. Defaults to `false`.
    pub enable_api: bool,
    /// The HMAC secret used for signing and verifying JWT tokens. This is sensitive information.
    pub jwt_secret: Option<SensitiveValue<String>>,
    /// If the API task should be spawned with the /metrics routes enabled. Defaults to `false`.
    pub enable_metrics: bool,
    /// What events should the API be subscribed to. Defaults to `all events`.
    #[serde(skip_deserializing, skip_serializing)]
    pub on: Vec<Trigger>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self::new(SocketAddr::from(([0, 0, 0, 0], 4269)), false, false, None)
    }
}

impl ApiConfig {
    pub fn new(
        bind_ip: SocketAddr,
        enable_api: bool,
        enable_metrics: bool,
        jwt_key: Option<String>,
    ) -> Self {
        Self {
            bind_ip,
            enable_api,
            on: Self::default_triggers(),
            enable_metrics,
            jwt_secret: jwt_key.map(SensitiveValue::new),
        }
    }

    /// The default triggers for the API task. It should be all events.
    fn default_triggers() -> Vec<Trigger> {
        let trigger = Trigger { rule: "origin > 0".to_owned(), interval: 1 };
        vec![trigger]
    }
}
