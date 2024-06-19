use color_eyre::{eyre, Report};
use reqwest::header::InvalidHeaderValue;
use slack_morphism::errors::SlackClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvmActionError {
    #[error("Failed to create the Action from config")]
    CreateFromConfig,
    #[error("Failed to compile the Solidity contracts")]
    CompileScript { source: Report },
    #[error("Failed to find the action source files")]
    SourceNotFound,
    #[error("Failed to execute the script")]
    ExecuteScript { source: Report },
    #[error("Failed to read the execution artifacts")]
    ArtifactRead { source: std::io::Error },
    #[error("Failed to parse '{0}' from the execution artifacts")]
    ArtifactParse(String),
}

#[derive(Error, Debug)]
pub enum WebhookError {
    #[error("Error while building the webhook request")]
    WebhookBuildError {
        #[from]
        source: eyre::Report,
    },
    #[error("HTTP request failed")]
    RequestFailed {
        #[from]
        source: reqwest::Error,
    },
    #[error("Invalid HTTP method")]
    InvalidMethod,
    #[error("Slack API error")]
    SlackError {
        #[from]
        source: SlackClientError,
    },
    #[error("Discord error")]
    DiscordError {
        #[from]
        source: serenity::Error,
    },
    #[error("Method not Implemented -- this is a bug")]
    NotImplemented,

    #[error("Webhook's target is not a valid url: {source}")]
    InvalidUrl {
        #[from]
        source: url::ParseError,
    },
    #[error("Invalid event Substitution")]
    InvalidSubst {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl From<InvalidHeaderValue> for WebhookError {
    fn from(error: InvalidHeaderValue) -> Self {
        Self::WebhookBuildError { source: error.into() }
    }
}
