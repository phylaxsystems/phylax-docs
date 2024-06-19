use crate::{
    substitute_env::{deserialize_seq_with_env, deserialize_with_env},
    SensitiveValue, Trigger,
};
use eyre::Result;
use http::Method;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

/// Generic config of an action
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionConfig {
    /// The name of the action
    #[serde(deserialize_with = "deserialize_with_env")]
    pub name: String,
    /// An array of events which will triger the action
    pub on: Vec<Trigger>,
    /// The type of the action
    #[serde(flatten)]
    pub kind: ActionKind,
    /// The minimum amount of time to wait between executions of the action
    /// (in seconds)
    #[serde(default)]
    pub delay: u32,
}

impl ActionConfig {
    /// Defaults to `./actions`, in relation to the root of the Phylax config
    fn default_project_root() -> PathBuf {
        "./actions".into()
    }

    /// Updates the paths in the configuration to be absolute paths
    pub fn with_canonicalized_paths(&mut self, root: &Path) -> Result<()> {
        #[allow(clippy::single_match)]
        match &mut self.kind {
            ActionKind::Evm(conf) => {
                let new_path = root.join(&conf.foundry_project_root_path);
                if !new_path.exists() {
                    return Err(eyre::eyre!("Path does not exist: {:?}", new_path));
                }
                conf.foundry_project_root_path = new_path;
            }
            _ => {}
        }
        Ok(())
    }
}
//
/// The type of the action. Either a forge script execution or
/// calling a remote endpoint API via a webhook
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum ActionKind {
    Evm(EvmActionConfig),
    ChatWebhook(ChatWebhookConfig),
    GeneralWebhook(GeneralWebhookConfig),
}

/// The specific config of an action that is a forge script execution
#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvmActionConfig {
    /// The name of the script's contract
    #[serde(deserialize_with = "deserialize_with_env")]
    pub contract_name: String,
    /// The signature of the entrypoint function.                                 
    /// If `None`, it's assumed it's `run()`                                          
    #[serde(default = "EvmActionConfig::default_signature")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub function_signature: String,
    /// The arguments of the entrypoint function                                      
    /// It supports the use of ENV variables as such: `$ENV_VARIABLE`
    #[serde(default = "EvmActionConfig::default_arguments")]
    #[serde(deserialize_with = "deserialize_seq_with_env")]
    pub function_arguments: Vec<String>,
    /// The configuration for the wallet and address that will be used to send the action's
    /// transactions It supports the use of ENV variables as such: `$ENV_VARIABLE`
    #[serde(deserialize_with = "deserialize_with_env")]
    pub wallet: SensitiveValue<WalletKind>,
    /// The root path of the forge project
    #[serde(default = "ActionConfig::default_project_root")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub foundry_project_root_path: PathBuf,
    #[serde(default = "EvmActionConfig::default_profile")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub profile: String,
}

impl EvmActionConfig {
    fn default_signature() -> String {
        "run()".into()
    }

    fn default_profile() -> String {
        "default".to_owned()
    }

    fn default_arguments() -> Vec<String> {
        vec![]
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
/// Type of Wallet that will be used by a `ForgeScriptAction`
pub enum WalletKind {
    /// Wallet uses a raw private key
    PrivateKey(String),
    /// Wallet uses AWS KMS to sign
    /// Environment variables it expects to find:
    /// - AWS_ACCESS_KEY_ID
    /// - AWS_SECRET_ACCESS_KEY
    /// - AWS_DEFAULT_REGION / AWS_REGION
    /// - AWS_KMS_KEY_ID / AWS_KMS_KEY_IDS (multiwallet)
    /// Documentation for the above AWS env variables can be found here:
    /// - <https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html>
    /// - <https://docs.aws.amazon.com/kms/latest/developerguide/find-cmk-id-arn.html>
    #[serde(rename = "aws")]
    #[default]
    Aws,
}
impl FromStr for WalletKind {
    type Err = std::string::ParseError;

    /// Parses a string s to return a WalletKind
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Aws" | "aws" => Ok(WalletKind::Aws),
            _ => Ok(WalletKind::PrivateKey(s.to_owned())),
        }
    }
}

/// the specific config of an action that is a webhook
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralWebhookConfig {
    /// the endpoint to be hit by the webhook
    /// It supports the use of ENV variables as such: `$ENV_VARIABLE`
    #[serde(deserialize_with = "deserialize_with_env")]
    pub target: String,
    /// what method to be used by the webhook
    /// defaults to `POST`
    #[serde(with = "http_serde::method")]
    pub method: Method,
    /// custom user payload to be added to the request's body
    /// defaults to `""`
    /// It supports the use of ENV variables as such: `$ENV_VARIABLE`
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub payload: String,
    /// authentication that endpoint expects
    /// defaults to `None`
    /// It supports the use of ENV variables as such: `$ENV_VARIABLE`
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub authentication: SensitiveValue<WebhookAuth>,
}

/// the specific config of an action that is a webhook
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatWebhookConfig {
    #[serde(default = "ChatWebhookConfig::default_avatar_url")]
    /// The URL for the avatar to be used by the bot when posting in the platform. This is only
    /// used by Discord, as in Slack the avatar of the bot is defined inside the Slack app
    /// directory
    pub avatar_url: Url,
    /// The description of the message
    #[serde(default = "ChatWebhookConfig::default_description")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub description: String,
    /// The platform: slack | discord
    pub platform: ChatPlatform,
    /// It supports the use of ENV variables as such: `$ENV_VARIABLE`
    #[serde(deserialize_with = "deserialize_with_env")]
    pub target: SensitiveValue<String>,
    /// The title of the message that is posted by the bot
    #[serde(default = "ChatWebhookConfig::default_title")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub title: String,
    /// The username of the bot. This is only used by Discord as in Slack, the name of the bot
    /// is defined inside the Slack app directory
    #[serde(default = "ChatWebhookConfig::default_username")]
    #[serde(deserialize_with = "deserialize_with_env")]
    pub username: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ChatPlatform {
    #[serde(rename = "discord")]
    Discord,
    #[serde(rename = "slack")]
    Slack,
}
impl ChatWebhookConfig {
    // TODO(odysseas): replace placeholder
    fn default_avatar_url() -> Url {
        Url::parse("https://pbs.twimg.com/profile_images/1686753859991949312/EgMf-fkb_400x400.jpg")
            .unwrap()
    }

    fn default_description() -> String {
        String::from("The webhook is a configurable task of Phylax")
    }

    fn default_title() -> String {
        String::from("Phylax Webhook")
    }

    fn default_username() -> String {
        String::from("Phylax")
    }
}

/// The types of authentication a webhook can have
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum WebhookAuth {
    #[default]
    None,
    /// Bearer token
    Bearer(String),
    /// Username and Password
    Basic { username: String, password: String },
}

impl FromStr for WebhookAuth {
    type Err = std::string::String;

    /// Parses a string s to return a WebhookAuth
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "None" => Ok(WebhookAuth::None),
            _ if s.starts_with("Bearer ") => Ok(WebhookAuth::Bearer(s[7..].to_string())),
            _ if s.contains(':') => {
                let parts: Vec<&str> = s.splitn(2, ':').collect();
                Ok(WebhookAuth::Basic {
                    username: parts[0].to_string(),
                    password: parts[1].to_string(),
                })
            }
            _ => Err(format!("Invalid string for WebhookAuth: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::try_absolute_path_of;
    use serde_json::json;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_default_project_root() {
        let default_path = ActionConfig::default_project_root();
        assert_eq!(default_path, Path::new("./actions"));
    }

    #[test]
    fn test_with_canonicalized_paths() {
        let temp_dir = tempdir().expect("unable to create temporary directory");
        let evm = EvmActionConfig {
            contract_name: String::from("Test Contract"),
            function_signature: String::from("run()"),
            function_arguments: vec![String::from("arg1"), String::from("arg2")],
            wallet: SensitiveValue::new(WalletKind::Aws),
            foundry_project_root_path: temp_dir.path().to_path_buf(),
            profile: "default".to_owned(),
        };
        let mut action_config = ActionConfig {
            name: "Test Action".to_string(),
            on: vec![],
            kind: ActionKind::Evm(evm),
            delay: 0,
        };
        let root_path = try_absolute_path_of(Path::new(".")).expect("unable to get absolute path");
        let result = action_config.with_canonicalized_paths(&root_path);
        assert!(result.is_ok());
        match action_config.kind {
            ActionKind::Evm(conf) => {
                assert_eq!(conf.foundry_project_root_path, root_path.join(temp_dir.path()));
            }
            _ => panic!("Unexpected ActionKind"),
        }
    }

    #[test]
    fn test_with_nonexistent_paths() {
        let evm = EvmActionConfig {
            profile: "default".to_owned(),
            contract_name: String::from("Test Contract"),
            function_signature: String::from("run()"),
            function_arguments: vec![String::from("arg1"), String::from("arg2")],
            wallet: SensitiveValue::new(WalletKind::Aws),
            foundry_project_root_path: "./nonexistent".into(),
        };
        let mut action_config = ActionConfig {
            name: "Test Action".to_string(),
            on: vec![],
            kind: ActionKind::Evm(evm),
            delay: 0,
        };
        let root_path = try_absolute_path_of(Path::new(".")).unwrap();
        let result = action_config.with_canonicalized_paths(&root_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_sub_env_vars_evm_action() {
        let evm_json = json!({
            "contractName": "Test Contract",
            "profile": "default",
            "functionSignature": "run()",
            "functionArguments": ["$ENV_VAR"],
            "wallet": "$ENV_VAR",
            "foundryProjectRootPath": "./"
        });
        std::env::set_var("ENV_VAR", "test");
        let evm: EvmActionConfig = serde_json::from_value(evm_json).unwrap();
        assert_eq!(evm.function_arguments[0], "test");
        match evm.wallet.into_inner() {
            WalletKind::PrivateKey(key) => assert_eq!(key, "test"),
            _ => panic!("Unexpected WalletKind"),
        }
    }
}
