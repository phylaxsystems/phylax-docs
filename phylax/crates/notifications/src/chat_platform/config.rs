use phylax_config::{sensitive::SensitiveValue, substitute_env::ValOrEnvVar};
use serde::{Deserialize, Serialize};

/// Configuration for a chat webhook action.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatWebhookConfig {
    /// The chat platform type (e.g., Slack, Discord, Telegram).
    pub platform: ChatPlatform,
    /// The target URL or an environment variable that holds the URL where the webhook will send
    /// data. It supports the use of environment variables as such: `$ENV_VARIABLE`.
    pub target: ValOrEnvVar<SensitiveValue<String>>,
}

/// Enum representing supported chat platforms.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatPlatform {
    /// Represents the Discord platform.
    #[serde(rename = "discord")]
    Discord,
    /// Represents the Slack platform.
    #[serde(rename = "slack")]
    Slack,
    /// Represents the Telegram platform.
    #[serde(rename = "telegram")]
    Telegram,
}
