use super::{error::TaskManagerError, task_handles::TaskHandles};
use futures_util::future::{join_all, select_all};
use phylax_tracing::tracing::debug;
use sisyphus_tasks::{sisyphus::TaskStatus, Sisyphus};
use std::collections::HashSet;
use tokio::{
    select,
    sync::{oneshot, watch::error::RecvError},
};

/// The task controller is more of an intermediary struct that we use to spawn the task manager.
// TODO(odyssseas): This seems like code smell. I should revisit this
pub struct TaskController {
    pub task_manager: Option<TaskManager>,
    /// shutdown the TaskController and all the spawned tasks
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    /// callback to let the orchestrator know that all tasks have been closed
    pub shutdown_cb_rx: Option<oneshot::Receiver<()>>,
    /// channel for the TaskStatus changes
    pub status_rx: flume::Receiver<(u32, TaskStatus)>,
}

impl TaskController {
    /// Create a new Task Controller
    pub fn new(
        task_manager: TaskManager,
        status_rx: flume::Receiver<(u32, TaskStatus)>,
        shutdown_tx: oneshot::Sender<()>,
        shutdown_cb_rx: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            task_manager: Some(task_manager),
            status_rx,
            shutdown_tx: Some(shutdown_tx),
            shutdown_cb_rx: Some(shutdown_cb_rx),
        }
    }
}
impl Default for TaskController {
    fn default() -> Self {
        Self {
            task_manager: None,
            status_rx: flume::unbounded().1,
            shutdown_tx: None,
            shutdown_cb_rx: None,
        }
    }
}

/// The Task Manager is responsible for watching over the tasks and letting the orchestrator
/// know when a task changes [`TaskStatus`].
pub struct TaskManager {
    /// A mapping between the id and [`sisyphus::Sisyphus`] handle of every task
    pub tasks: TaskHandles,
    /// Shutdown signal
    pub shutdown_rx: oneshot::Receiver<()>,
    /// Shutdown signal callback so the orchestrator knows when the shutdown has been complete
    pub shutdown_cb: oneshot::Sender<()>,
    /// The channel for sending the new TaskStatus whenever a task changes status
    pub status_tx: flume::Sender<(u32, TaskStatus)>,
}

impl TaskManager {
    pub fn new(
        tasks: TaskHandles,
        shutdown_rx: oneshot::Receiver<()>,
        shutdown_cb: oneshot::Sender<()>,
        status_tx: flume::Sender<(u32, TaskStatus)>,
    ) -> Self {
        debug!("Created Task Manager");
        Self { tasks, shutdown_rx, status_tx, shutdown_cb }
    }
    pub async fn watch_task_status(
        id: u32,
        task: &mut Sisyphus,
    ) -> (u32, &mut Sisyphus, Result<TaskStatus, RecvError>) {
        let result = task.watch_status().await;
        (id, task, result)
    }
    /// The main loop of the Task Manager. It watches over the tasks and if their status is updated,
    /// it signals the orchestrator, which in turns emits an event in the event bus.
    /// It also handles the shutdown signal and cleans up the tasks.
    pub(crate) async fn main_loop(mut self) -> Result<(), TaskManagerError> {
        // A set to keep track of removed tasks
        // let mut removed = HashMap::new();
        // A vector to hold the futures of the s
        // Iterate over the tasks and add their futures to the vector
        let mut removed = HashSet::new();
        loop {
            let mut futs = Vec::new();
            self.tasks.0.iter_mut().for_each(|(id, task)| {
                // If the task is not in the removed set, add its future to the vector
                if removed.get(id).is_none() {
                    futs.push(Box::pin(Self::watch_task_status(*id, task)));
                }
            });
            // Select over the futures and the shutdown receiver
            select! {
               // If the shutdown signal is received, clean up the tasks
               _ = &mut self.shutdown_rx => {
                   debug!("Task Manager received shutdown signal");
                   // Drop the futures to cancel m
                   drop(futs);
                   // A vector to hold the shutdown handles of the tasks
                   let mut handles = Vec::new();
                   // Iterate over the tasks and send them the shutdown signal
                   while let Some((_id, task)) = self.tasks.pop_first() {
                       // Send shutdown signal
                       let handle = task.shutdown();
                       // Add the shutdown handle to the vector
                       handles.push(handle);
                   }
                   // Await the shutdown handles to ensure all tasks are shut down
                   join_all(handles).await;
                   debug!("All tasks have been shut down");
                   // Alert the Orchestrator so it can finish cleaning up
                   self.shutdown_cb.send(()).map_err(|_|TaskManagerError::TaskShutdownCallback)?;
                   return Ok(());
               }
               // If a task changes its status, update it and replace its future
               ((id, _task, status_or_err), _index, _remaining) = select_all(&mut futs) => {
                   match status_or_err{
                       Ok(status) => {
                           debug!(task_id=id, ?status, "Task Manager detected a new task state");
                           // Send the new status to the status channel
                           self.status_tx.send((id, status)).map_err(|_e| TaskManagerError::TaskStatusSend(id))?;
                       }
                       Err(_) => {
                           // If the task fails, it will shutdown and drop the sending channel,
                           // which will produce the error here. We don't do anything about the error.
                           debug!(task_id=id, "Task status channel was closed");
                           // Add the task to the removed set
                           removed.insert(id);
                       }
                   }
               }
               // Flush the removed_set every 3 seconds
               _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
                   removed = HashSet::new();
               }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{activities::system::orchestrator::Orchestrator, mocks::mock_task::MockSisyphus};
    use phylax_config::PhConfig;
    use sisyphus_tasks::Boulder;
    use std::sync::Arc;
    use tokio::{sync::mpsc, time::Duration};
    use tracing_test::traced_test;

    fn setup() -> Orchestrator {
        let config = PhConfig::default();
        Orchestrator::new(Arc::new(config))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_task_manager_watch_task_status() {
        let (tx, rx) = mpsc::channel(1);
        let mut handle = MockSisyphus::new(rx).run_until_panic();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            tx.send("panic".into()).await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        let id = 1;
        let (task_id, _, result) = TaskManager::watch_task_status(id, &mut handle).await;
        assert_eq!(task_id, id);
        assert_eq!(result.unwrap().to_string(), TaskStatus::Running.to_string());

        let (task_id, _, result) = TaskManager::watch_task_status(id, &mut handle).await;
        assert_eq!(task_id, id);
        assert_eq!(
            result.unwrap().to_string(),
            TaskStatus::Stopped {
                exceptional: true,
                err: Arc::new(eyre::Report::msg("big if true"))
            }
            .to_string()
        );
    }

    #[tokio::test]
    async fn test_task_manager_main_loop_shutdown_itself_and_tasks() {
        // Arrange
        let orchestrator = setup();
        let (tx, rx) = mpsc::channel(1);
        let mut handles = TaskHandles::default();
        let handle = MockSisyphus::new(rx).run_until_panic();
        handles.insert(1, handle);
        let mut controller = orchestrator.build_task_controller(handles);

        let manager = controller.task_manager.take().unwrap();
        let manager_handle = tokio::spawn(manager.main_loop());
        assert!(!tx.is_closed());
        assert!(controller.shutdown_tx.unwrap().send(()).is_ok());
        assert!(manager_handle.await.is_ok());
        assert!(tx.is_closed());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[traced_test]
    async fn test_task_manager_handle_task_restart() {
        // Arrange
        let orchestrator = setup();
        let (tx, rx) = mpsc::channel(1);
        let (tx2, rx2) = mpsc::channel(1);
        let mut handles = TaskHandles::default();
        let handle1 = MockSisyphus::new(rx).run_until_panic();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let handle2 = MockSisyphus::new(rx2).run_until_panic();
        handles.insert(1, handle1);
        handles.insert(2, handle2);
        let mut controller = orchestrator.build_task_controller(handles);

        let status_rx = controller.status_rx;

        let manager = controller.task_manager.take().unwrap();
        assert_eq!(status_rx.len(), 0);
        tokio::spawn(manager.main_loop());
        assert_eq!(status_rx.len(), 0);
        assert!(!tx.is_closed());

        // task 1
        let (id1, new_status) = status_rx.recv().unwrap();
        assert_eq!(new_status.to_string(), TaskStatus::Running.to_string());
        assert_eq!(id1, 1);
        // task 2
        let (id2, new_status) = status_rx.recv().unwrap();
        assert_eq!(new_status.to_string(), TaskStatus::Running.to_string());
        assert_eq!(id2, 2);

        //restart task 1
        tx.send("restart".into()).await.unwrap();

        let (id1, new_status) = status_rx.recv().unwrap();
        assert_eq!(
            new_status.to_string(),
            TaskStatus::Recovering(Arc::new(eyre::Report::msg("coming right back"))).to_string()
        );
        assert_eq!(id1, 1);

        let (id1, new_status) = status_rx.recv().unwrap();
        assert_eq!(new_status.to_string(), TaskStatus::Running.to_string());
        assert_eq!(id1, 1);

        tx2.send("restart".into()).await.unwrap();

        let (id2, new_status) = status_rx.recv().unwrap();
        assert_eq!(
            new_status.to_string(),
            TaskStatus::Recovering(Arc::new(eyre::Report::msg("coming right back"))).to_string()
        );
        assert_eq!(id2, 2);

        let (id2, new_status) = status_rx.recv().unwrap();
        assert_eq!(new_status.to_string(), TaskStatus::Running.to_string());
        assert_eq!(id2, 2);

        assert_eq!(status_rx.len(), 0);
    }
}
