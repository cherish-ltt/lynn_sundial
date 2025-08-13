use std::{sync::Arc, thread::sleep, time::Duration};

use chrono::Local;
use crossbeam_deque::Injector;
use tokio::task::JoinHandle;

use crate::schedule::{
    config::DEFAULT_TICK_TIME,
    task::{ITaskHandler, TaskPollTrait},
    time_wheel::TierTimeWheel,
};

pub(super) struct CoreReactor {
    pub(super) core_join_handle: Option<JoinHandle<()>>,
}

impl CoreReactor {
    pub(crate) fn new() -> Self {
        Self {
            core_join_handle: None,
        }
    }

    pub(crate) fn start<T>(
        &mut self,
        time_wheel: Arc<TierTimeWheel<T>>,
        global_queue: Arc<Injector<Arc<Box<dyn ITaskHandler>>>>,
    ) where
        T: TaskPollTrait + 'static,
    {
        let core_join_handle = tokio::spawn(async move {
            let time_wheel = time_wheel;
            let mut tick_detal = DEFAULT_TICK_TIME;
            loop {
                let start_time = Local::now();
                sleep(Duration::from_millis(DEFAULT_TICK_TIME));
                let handle_vec = time_wheel.tick(tick_detal).await;
                for handle in handle_vec {
                    global_queue.push(handle);
                }
                let end_time = Local::now();
                tick_detal = end_time
                    .signed_duration_since(start_time)
                    .num_milliseconds() as u64;
            }
        });
        self.core_join_handle = Some(core_join_handle);
    }
}
