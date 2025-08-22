use std::sync::Arc;

use tokio::sync::RwLock;

use crate::schedule::{
    reactor::{core_reactor::CoreReactor, task_reactor::TasksManager},
    task_actor::TaskStatus,
    time_wheel::TierTimeWheel,
};

mod core_reactor;
mod task_reactor;

/// ## 任务中心
pub(crate) struct TaskReactor {
    core_reactor: CoreReactor,
    task_manager: TasksManager,
}

impl TaskReactor {
    pub(crate) fn new() -> Self {
        Self {
            core_reactor: CoreReactor::new(),
            task_manager: TasksManager::new(),
        }
    }

    pub(crate) async fn wait_all(&mut self) -> Result<(), tokio::task::JoinError> {
        if let Some(handle) = self.core_reactor.core_join_handle.as_mut() {
            //handle.
            handle.await
        } else {
            Ok(())
        }
    }

    pub(crate) fn start(
        &mut self,
        time_wheel: Arc<TierTimeWheel>,
        notice_list: Arc<RwLock<Option<Vec<(usize, TaskStatus)>>>>,
    ) {
        self.task_manager.start();
        self.core_reactor.start(
            time_wheel,
            self.task_manager.get_global_queue(),
            notice_list,
        );
    }
}
