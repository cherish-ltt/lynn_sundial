use std::{collections::HashMap, sync::Arc};

use cron::Schedule;
use tokio::sync::{
    RwLock,
    mpsc::{Sender, channel},
};

use crate::schedule::task_actor::{TaskActor, TaskSignal, TaskStatus};

pub(crate) struct TaskManager {
    /// task_id计数器
    pub(crate) task_id_counter: usize,
    /// id-task的映射
    pub(crate) id_task_mapping: Option<HashMap<usize, Sender<TaskSignal>>>,
    /// 被暂停的task
    pub(crate) idle_task: Option<HashMap<usize, TaskActor>>,
    /// notice_list
    pub(crate) notice_list: Arc<RwLock<Option<Vec<(usize, TaskStatus)>>>>,
}

impl TaskManager {
    pub(crate) fn new() -> Self {
        Self {
            task_id_counter: 0,
            id_task_mapping: None,
            idle_task: None,
            notice_list: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) fn insert_new_task(&mut self, task_id: usize, sender: Sender<TaskSignal>) {
        if let None = self.id_task_mapping.as_mut() {
            self.id_task_mapping = Some(HashMap::new());
        }
        if let Some(map) = self.id_task_mapping.as_mut() {
            map.insert(task_id, sender);
        }
    }

    pub(crate) fn get_new_id(&mut self) -> Option<usize> {
        if self.task_id_counter < usize::MAX {
            self.task_id_counter += 1;
            return Some(self.task_id_counter.clone());
        }
        None
    }

    pub(crate) fn get_notice_list(&self) -> Arc<RwLock<Option<Vec<(usize, TaskStatus)>>>> {
        self.notice_list.clone()
    }

    pub(crate) async fn update_task_status_by_id(
        &mut self,
        task_id: usize,
        task_status: TaskStatus,
    ) -> bool {
        if let Some(map) = self.id_task_mapping.as_ref() {
            if map.contains_key(&task_id) {
                match task_status {
                    TaskStatus::Pause => {
                        if let Some(idle_task) = self.idle_task.as_ref() {
                            if idle_task.contains_key(&task_id) {
                                return true;
                            }
                        }
                        if let Some(sender) = map.get(&task_id) {
                            let (tx, mut rx) = channel(1);
                            if let Ok(()) = sender.send(TaskSignal::Pause(tx)).await {
                                if let Some(task_actor) = rx.recv().await {
                                    if let Some(idle_task) = self.idle_task.as_mut() {
                                        idle_task.insert(task_id, task_actor);
                                    } else {
                                        let mut map = HashMap::new();
                                        map.insert(task_id, task_actor);
                                        self.idle_task = Some(map);
                                    }
                                    let mut mutex = self.notice_list.write().await;
                                    if let Some(vec) = mutex.as_mut() {
                                        vec.push((task_id, task_status));
                                    } else {
                                        *mutex = Some(vec![(task_id, task_status)]);
                                    }
                                    return true;
                                }
                            }
                        }
                    }
                    TaskStatus::Destory => {
                        if let Some(sender) = map.get(&task_id) {
                            if let Ok(()) = sender.send(TaskSignal::Destory).await {
                                let mut mutex = self.notice_list.write().await;
                                if let Some(vec) = mutex.as_mut() {
                                    vec.push((task_id, task_status));
                                } else {
                                    *mutex = Some(vec![(task_id, task_status)]);
                                }
                                return true;
                            }
                        }
                    }
                    TaskStatus::Running => {
                        if let Some(idle_task) = self.idle_task.as_mut() {
                            if idle_task.contains_key(&task_id) {
                                if let Some(task_actor) = idle_task.remove(&task_id) {
                                    task_actor.start_actor();
                                    let mut mutex = self.notice_list.write().await;
                                    if let Some(vec) = mutex.as_mut() {
                                        vec.push((task_id, task_status));
                                    } else {
                                        *mutex = Some(vec![(task_id, task_status)]);
                                    }
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub(crate) async fn update_cron_by_id(&mut self, task_id: usize, cron: Schedule) -> bool {
        if let Some(id_task_mapping) = self.id_task_mapping.as_ref() {
            if id_task_mapping.contains_key(&task_id) {
                if let Some(sender) = id_task_mapping.get(&task_id) {
                    if let Ok(()) = sender.send(TaskSignal::UpdateCron(cron)).await {
                        return true;
                    }
                }
            }
        }
        false
    }
}
