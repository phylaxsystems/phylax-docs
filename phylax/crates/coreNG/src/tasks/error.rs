use thiserror::Error;

use crate::{activities::ActivityError, error::EventBusError};

use super::subscriptions::SubscriptionsError;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("The Task couldn't find {0} in the task DNS")]
    TaskDns(String),
    #[error("The Task encountered an error with the event bus")]
    EventBusError {
        #[from]
        source: EventBusError,
    },
    #[error("The Task encountered an error with the Activity")]
    ActivityError {
        #[from]
        source: ActivityError,
    },
    #[error("The Task encountered an error while bootstraping the Activity")]
    BootstrapError { source: eyre::Report },
    #[error("The background work encountered an error")]
    BgWorkError { source: eyre::Report },
    #[error("The background work panicked or encountered an unhandled error")]
    BgHandleError,
    #[error("The foreground work panicked or encountered an unhandled error")]
    FgHandleError,
    #[error("The foreground work encountered an error")]
    FgWorkError { source: eyre::Report },
    #[error("The Task encountered an error with the event buffer")]
    EventBufferError { source: Box<dyn std::error::Error + Send + Sync + 'static> },
    #[error("The Task encountered an error while doing work in the foreground")]
    FgWork { source: eyre::Report },
    #[error("The Task's internal event stream closed abruptly")]
    InternalStream,
    #[error("The Task encountered an error with the DNS: {source}")]
    DnsError { source: eyre::Report },
    #[error("The Task encountered while evaluating an event with the subscription")]
    SubscriptionError {
        #[from]
        source: SubscriptionsError,
    },
}

#[derive(Error, Debug)]
pub enum SpawnError {
    #[error("Failed to create subscriptions from configuration triggers")]
    Subscriptions {
        #[from]
        source: eyre::Report,
    },
    #[error("The Task {0} does not exist in the configuration")]
    TaskNotFound(String),
    #[error("Failed to spawn a task from the config:{0:?} with error: {1}")]
    InvalidConfig(Box<dyn std::fmt::Debug + Send + Sync>, Box<dyn std::error::Error + Send + Sync>),
}

impl PartialEq for SpawnError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SpawnError::Subscriptions { source: _ }, SpawnError::Subscriptions { source: _ }) => {
                true
            }
            (SpawnError::TaskNotFound(a), SpawnError::TaskNotFound(b)) => a == b,
            (SpawnError::InvalidConfig(a, _), SpawnError::InvalidConfig(b, _)) => {
                format!("{:?}", a) == format!("{:?}", b)
            }
            _ => false,
        }
    }
}
