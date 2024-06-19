use serde::{Deserialize, Serialize};

use crate::error::ApiInternalBusError;

#[derive(Debug, Clone)]
pub struct ApiInternalBus {
    pub cmd_tx: flume::Sender<ApiInternalCmd>,
    pub cmd_rx: flume::Receiver<ApiInternalCmd>,
    pub events_tx: flume::Sender<SystemEvent>,
    pub events_rx: flume::Receiver<SystemEvent>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ApiInternalCmd {
    ActivateTask(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SystemEvent {
    pub origin: String,
    pub event_type: String,
    pub timestamp: u64,
    pub body: serde_json::Value,
}

impl ApiInternalBus {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = flume::unbounded();
        let (events_tx, events_rx) = flume::unbounded();
        Self { cmd_tx, cmd_rx, events_tx, events_rx }
    }

    pub async fn async_recv_cmd(
        &self,
    ) -> Result<ApiInternalCmd, ApiInternalBusError<ApiInternalCmd>> {
        self.cmd_rx.recv_async().await.map_err(|e| e.into())
    }

    pub fn send_cmd(&self, cmd: ApiInternalCmd) -> Result<(), ApiInternalBusError<ApiInternalCmd>> {
        self.cmd_tx.send(cmd).map_err(|e| e.into())
    }

    pub async fn async_recv_event(&self) -> Result<SystemEvent, ApiInternalBusError<SystemEvent>> {
        self.events_rx.recv_async().await.map_err(|e| e.into())
    }

    pub fn send_event(
        &self,
        event: impl Into<SystemEvent>,
    ) -> Result<(), ApiInternalBusError<SystemEvent>> {
        self.events_tx.send(event.into()).map_err(|e| e.into())
    }
}

impl Default for ApiInternalBus {
    fn default() -> Self {
        Self::new()
    }
}
