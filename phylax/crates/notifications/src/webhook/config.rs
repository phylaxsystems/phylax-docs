use http::Method;
use phylax_config::{sensitive::SensitiveValue, substitute_env::ValOrEnvVar};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Configuration for a general webhook action.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneralWebhookConfig {
    /// The endpoint URL where the webhook will send data. Supports environment variable
    /// substitution.
    pub target: ValOrEnvVar<SensitiveValue<String>>,
    /// HTTP method to be used when the webhook is triggered. Defaults to `POST`.
    #[serde(with = "http_serde::method")]
    pub method: Method,
    /// Custom payload to be sent in the request body. Supports environment variable substitution.
    /// Defaults to an empty string.
    #[serde(default)]
    pub payload: ValOrEnvVar<String>,
    /// Authentication method required by the endpoint. Supports environment variable substitution.
    /// Defaults to `None`.
    #[serde(default)]
    pub authentication: WebhookAuth,
}

/// Defines the types of authentication supported by a webhook.
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum WebhookAuth {
    /// No authentication required.
    #[default]
    None,
    /// Bearer token authentication.
    Bearer(ValOrEnvVar<SensitiveValue<String>>),
    /// Basic authentication with username and password.
    Basic {
        /// The username for basic authentication. Supports environment variable substitution.
        username: ValOrEnvVar<SensitiveValue<String>>,
        /// The password for basic authentication. Supports environment variable substitution.
        password: ValOrEnvVar<SensitiveValue<String>>,
    },
}

impl FromStr for WebhookAuth {
    type Err = std::string::String;

    /// Parses a string to return a `WebhookAuth` variant.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "None" | "none" => Ok(WebhookAuth::None),
            _ if s.starts_with("Bearer ") => {
                let token = ValOrEnvVar::new(SensitiveValue::new(s[7..].to_string()));
                Ok(WebhookAuth::Bearer(token))
            }
            _ if s.contains(':') => {
                let parts: Vec<&str> = s.splitn(2, ':').collect();
                Ok(WebhookAuth::Basic {
                    username: ValOrEnvVar::new(SensitiveValue::new(parts[0].to_string())),
                    password: ValOrEnvVar::new(SensitiveValue::new(parts[1].to_string())),
                })
            }
            _ => Err(format!("Invalid string for WebhookAuth: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_auth_from_str_none() {
        let auth = WebhookAuth::from_str("None").unwrap();
        assert_eq!(auth, WebhookAuth::None);
    }

    #[test]
    fn test_webhook_auth_from_str_bearer() {
        let auth_str = "Bearer abc123";
        let expected_token = ValOrEnvVar::new(SensitiveValue::new("abc123".to_string()));
        let auth = WebhookAuth::from_str(auth_str).unwrap();
        match auth {
            WebhookAuth::Bearer(token) => assert_eq!(token, expected_token),
            _ => panic!("Expected Bearer token"),
        }
    }

    #[test]
    fn test_webhook_auth_from_str_basic() {
        let auth_str = "user:pass";
        let expected_username = ValOrEnvVar::new(SensitiveValue::new("user".to_string()));
        let expected_password = ValOrEnvVar::new(SensitiveValue::new("pass".to_string()));
        let auth = WebhookAuth::from_str(auth_str).unwrap();
        match auth {
            WebhookAuth::Basic { username, password } => {
                assert_eq!(username, expected_username);
                assert_eq!(password, expected_password);
            }
            _ => panic!("Expected Basic auth"),
        }
    }

    #[test]
    fn test_webhook_auth_from_str_invalid() {
        let auth_str = "invalid_auth_format";
        assert!(WebhookAuth::from_str(auth_str).is_err());
    }
}
