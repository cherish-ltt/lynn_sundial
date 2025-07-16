use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::schedule::{
    reactor::{core_reactor::CoreReactor, task_reactor::TasksManager},
    task::{Task, TaskPollTrait},
    time_wheel::TierTimeWheel,
};

mod core_reactor;
mod task_reactor;

/// ## 任务中心
pub(crate) struct TaskReactor {
    core_join_handle: Option<JoinHandle<()>>,
}

impl TaskReactor {
    pub(crate) fn new() -> Self {
        Self {
            core_join_handle: None,
        }
    }

    pub(crate) fn start<T>(&mut self, time_wheel: Arc<TierTimeWheel<T>>)
    where
        T: TaskPollTrait + 'static,
    {
        let core_join_handle = tokio::spawn(async move {
            let time_wheel = time_wheel;
            loop {
                let handle_vec = time_wheel.tick().await;
                for handle in handle_vec {
                    handle.run().await;
                }
            }
        });
        self.core_join_handle = Some(core_join_handle);
    }
}
