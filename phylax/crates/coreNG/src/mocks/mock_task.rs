use std::time::Duration;

use eyre::Report;
use sisyphus_tasks::{Boulder, Fall};
use tokio::{pin, select, sync::mpsc};

pub struct MockSisyphus {
    cmds_sig: mpsc::Receiver<MockSisyphusCmds>,
}

pub enum MockSisyphusCmds {
    Stop,
    Restart,
}
impl std::str::FromStr for MockSisyphusCmds {
    type Err = ();

    /// Converts a string into a `MockSisyphusCmds`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "panic" => Ok(MockSisyphusCmds::Stop),
            "restart" => Ok(MockSisyphusCmds::Restart),
            _ => Err(()),
        }
    }
}

impl From<&str> for MockSisyphusCmds {
    /// Converts a string into a `MockSisyphusCmds`
    fn from(s: &str) -> Self {
        match s.parse() {
            Ok(cmd) => cmd,
            Err(_) => panic!("Invalid command"),
        }
    }
}

impl MockSisyphus {
    pub fn new(cmds_sig: mpsc::Receiver<MockSisyphusCmds>) -> Self {
        Self { cmds_sig }
    }
}

impl Boulder for MockSisyphus {
    fn spawn(
        mut self,
        mut shutdown: sisyphus_tasks::sisyphus::ShutdownSignal,
    ) -> tokio::task::JoinHandle<sisyphus_tasks::Fall<Self>>
    where
        Self: 'static + Send + Sync + Sized,
    {
        tokio::task::spawn(async move {
            println!("Mock Sisyphus task spawned");
            let sig = &mut self.cmds_sig;
            pin!(sig);
            select! {
                    biased;
                    _ = &mut shutdown => {
                        Fall::Shutdown { task: self}
                    }
                    Some(sig) = sig.recv() => {
                        match sig {
                            MockSisyphusCmds::Stop => Fall::Unrecoverable { exceptional: true, err: Report::msg("big if true"), task: self },
                            MockSisyphusCmds::Restart => Fall::Recoverable { task: self, err: Report::msg("coming right back"), shutdown },
                        }
                    }
            }
        })
    }

    fn restart_after_ms(&self) -> u64 {
        500
    }

    fn task_description(&self) -> String {
        format!("{self}")
    }

    fn cleanup(
        self,
    ) -> std::pin::Pin<Box<dyn futures::prelude::Future<Output = eyre::Result<()>> + Send>>
    where
        Self: 'static + Send + Sync + Sized,
    {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            Ok(())
        })
    }

    fn recover(
        self,
    ) -> std::pin::Pin<Box<dyn futures::prelude::Future<Output = eyre::Result<Self>> + Send>>
    where
        Self: 'static + Send + Sync + Sized,
    {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            Ok(self)
        })
    }
}

impl std::fmt::Display for MockSisyphus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MockSisyphus")
    }
}
