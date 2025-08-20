use std::{sync::Arc, time::Duration};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};

use crate::schedule::{
    config::{DEFAULT_TASK_POOL_SIZE, DEFAULT_TICK_TIME},
    task_actor::ITaskHandler,
};

pub(super) struct TasksManager {
    global_queue: Arc<Injector<Arc<Box<dyn ITaskHandler>>>>,
}

impl TasksManager {
    pub(crate) fn new() -> Self {
        let sender = Arc::new(Injector::new());
        Self {
            global_queue: sender,
        }
    }

    pub(crate) fn get_global_queue(&self) -> Arc<Injector<Arc<Box<dyn ITaskHandler>>>> {
        self.global_queue.clone()
    }

    pub(crate) fn start(&self) {
        let mut local_queues: Vec<Worker<Arc<Box<dyn ITaskHandler>>>> =
            Vec::with_capacity(DEFAULT_TASK_POOL_SIZE);
        let mut stealers: Vec<Stealer<Arc<Box<dyn ITaskHandler>>>> =
            Vec::with_capacity(DEFAULT_TASK_POOL_SIZE);
        for _ in 0..DEFAULT_TASK_POOL_SIZE {
            let worker = Worker::new_lifo();
            stealers.push(worker.stealer());
            local_queues.push(worker);
        }
        let global_queue = self.global_queue.clone();
        let stealers_arc = Arc::new(stealers);
        for local_queue in local_queues {
            let local_queue = local_queue;
            let global_queue = global_queue.clone();
            let stealers_arc = stealers_arc.clone();
            tokio::spawn(async move {
                loop {
                    if let Some(task) = get_task(&local_queue, &global_queue, &stealers_arc) {
                        task.run().await;
                    } else {
                        tokio::time::sleep(Duration::from_millis(DEFAULT_TICK_TIME)).await;
                    }
                }
            });
        }
    }
}

#[inline(always)]
fn get_task(
    local_queue: &Worker<Arc<Box<dyn ITaskHandler>>>,
    global_queue: &Arc<Injector<Arc<Box<dyn ITaskHandler>>>>,
    stealers_arc: &Arc<Vec<Stealer<Arc<Box<dyn ITaskHandler>>>>>,
) -> Option<Arc<Box<dyn ITaskHandler>>> {
    // 1. local
    if let Some(event) = local_queue.pop() {
        return Some(event);
    }

    // 2. global
    if let Steal::Success(event) = global_queue.steal_batch_and_pop(local_queue) {
        return Some(event);
    }

    // 3. stealers
    for i in 0..stealers_arc.len() {
        if let Steal::Success(event) = stealers_arc[i].steal() {
            return Some(event);
        }
    }

    None
}
