use crate::{args::parse_socket_address, tui::Tui};
use clap::Args;
use phylax_config::PhConfig;
use phylax_core::tasks::spawn::Spawnable;
use phylax_tracing::tracing::{error, info};
use serde::Serialize;
use sisyphus_tasks::{sisyphus::TaskStatus, Sisyphus};
use std::{net::SocketAddr, sync::Arc};

#[derive(Debug, Clone, Args, Serialize, Default)]
pub struct NodeCommand {
    /// The API and Metrics will be served at the given interface and port.
    #[arg(long, value_name = "SOCKET", value_parser = parse_socket_address, help_heading = "bind IP", help="The IP:PORT to bind the API to")]
    pub bind_ip: Option<SocketAddr>,
    #[arg(long, help_heading = "Enable API", help = "Whether the API be enabled or not")]
    pub enable_api: bool,
    #[arg(
        long,
        help_heading = "Enable Prometheus Metrics",
        help = "Whether the '/metrics endpoint should be enabled or not"
    )]
    pub enable_metrics: bool,
    #[arg(
        long,
        help_heading = "Enable Authentication",
        help = "The JWT secret that will be used to authenticate API requests"
    )]
    pub jwt_secret: Option<String>,
}

impl NodeCommand {
    /// Execute `node` command
    pub async fn execute(self, config: PhConfig) -> eyre::Result<()> {
        info!("I'm Here To Kick Ass And Chew Bubblegum");
        let config_arc = Arc::new(config);
        let orch_config = config_arc.orchestrator.clone();
        let artifacts = orch_config.spawn_task(
            Default::default(),
            Default::default(),
            Default::default(),
            config_arc,
        )?;
        let tui = Tui;
        let mut handle: Sisyphus = artifacts.handle;
        loop {
            tokio::select! {
                    _ = tui.detect_ctrl_c() => {
                        info!("Detected ctrl c. Phylax will shutdown");
                        handle.shutdown().await?;
                        return Ok(())
                    },
                    Ok(status) = handle.watch_status() => {
                        match status {
                            TaskStatus::Stopped { exceptional: true, err: error } => {
                                error!("Phylax encountered an error: {:?}", error);
                                info!("Phylax will now exit");
                                return Ok(());
                            }
                            TaskStatus::Panicked => {
                                error!("Phylax panicked. This is a bug");
                                info!("Phylax will now exit");
                                return Ok(());
                            }
                            _ => {}
                        }
                }
            }
        }
    }
}
