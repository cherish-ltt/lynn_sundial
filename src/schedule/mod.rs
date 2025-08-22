mod config;
mod reactor;
mod task_actor;
mod task_manager;
mod time_wheel;

use crate::schedule::config::{
    DEFAULT_ERROR_CODE_1000, DEFAULT_ERROR_CODE_1001, DEFAULT_ERROR_CODE_1002,
};
use crate::schedule::task_actor::{ITaskHandler, Task, TaskOrderType, TaskStatus};
#[cfg(feature = "schedule")]
use crate::schedule::task_manager::TaskManager;
use crate::schedule::{reactor::TaskReactor, time_wheel::TierTimeWheel};
use chrono::Local;
use std::error::Error;
use std::sync::Arc;
use std::{marker::PhantomData, pin::Pin, str::FromStr};

#[cfg(feature = "schedule")]
pub enum RepeatModel {
    /// 只运行一次
    Once,
    /// 重复运行
    Repetition,
    /// 运行指定次数
    Times(usize),
}

/// ## 定时任务调度器
#[cfg(feature = "schedule")]
pub struct Scheduler {
    pub(crate) time_wheel: Arc<TierTimeWheel>,
    pub(crate) task_reactor: TaskReactor,
    pub(crate) task_manager: TaskManager,
}

impl Scheduler {
    pub fn new() -> Self {
        let time_wheel = Arc::new(TierTimeWheel::new());
        let mut task_reactor = TaskReactor::new();
        let task_manager = TaskManager::new();
        task_reactor.start(time_wheel.clone(), task_manager.get_notice_list());

        Self {
            time_wheel,
            task_reactor,
            task_manager,
        }
    }

    /// #### 异步阻塞等待定时器
    /// 注意：定时器的内部reactor在new时已经启动，`wait_all`方法是用于阻塞主线程而额外提供的异步方法，你也可以在主线程使用类似`loop{}`来避免主线程提前结束（不推荐）
    pub async fn wait_all(&mut self) {
        let _ = self.task_reactor.wait_all().await;
    }

    /// #### 新增默认的有序定时任务
    /// - 使用`push_task`来新增等同于`push_order_task`的有序定时任务
    /// - 使用`push_order_task`来新增上一次任务A尚未结束时，下一次需要运行的任务A进行排队等候的定时任务
    /// - 使用`push_disorder_task`来新增无需关注上一次任务A是否结束，就允许新任务A运行的定时任务
    pub fn push_task(
        &mut self,
        cron: &str,
        handle: impl IntoSystem,
        repeat: RepeatModel,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        self.push_order_task(cron, handle, repeat)
    }

    /// #### 新增有序定时任务
    /// 注意：
    /// - 所有为`RepeatModel::Repetition/::Times(>0)`的同一定时任务A，上一次任务A尚未结束时，下一次需要运行的任务A进行排队等候的定时任务
    pub fn push_order_task(
        &mut self,
        cron: &str,
        handle: impl IntoSystem,
        repeat: RepeatModel,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if let Some(task_id) = self.task_manager.get_new_id() {
            let cron_schedule = cron::Schedule::from_str(cron)?;
            if let Some(next_time) = cron_schedule.upcoming(Local).next() {
                let now_time = Local::now();
                let time_delta = next_time.signed_duration_since(now_time);
                let milliseconds = time_delta.num_milliseconds();
                let task = Task::new(
                    cron_schedule,
                    Arc::new(Box::new(handle.to_system())),
                    repeat,
                    next_time,
                    TaskOrderType::Order,
                    task_id,
                );
                self.task_manager
                    .insert_new_task(task_id, task.get_sender());
                self.time_wheel.push_T_to_time_wheel(task, milliseconds);
                return Ok(task_id);
            }
            return Err(Box::new(SchedulerError(
                DEFAULT_ERROR_CODE_1001.to_string(),
            )));
        } else {
            return Err(Box::new(SchedulerError(
                DEFAULT_ERROR_CODE_1000.to_string(),
            )));
        }
    }

    /// #### 新增无序定时任务
    /// 注意：
    /// - 所有为`RepeatModel::Repetition/::Times(>0)`的同一定时任务，无需关注上一次任务是否结束就允许运行新的任务（如：任务A，无需关注上一次任务A是否结束，就允许新任务A运行的定时任务）
    pub fn push_disorder_task(
        &mut self,
        cron: &str,
        handle: impl IntoSystem,
        repeat: RepeatModel,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if let Some(task_id) = self.task_manager.get_new_id() {
            let cron_schedule = cron::Schedule::from_str(cron)?;
            if let Some(next_time) = cron_schedule.upcoming(Local).next() {
                let now_time = Local::now();
                let time_delta = next_time.signed_duration_since(now_time);
                let milliseconds = time_delta.num_milliseconds();
                let task = Task::new(
                    cron_schedule,
                    Arc::new(Box::new(handle.to_system())),
                    repeat,
                    next_time,
                    TaskOrderType::Disorder,
                    task_id,
                );
                self.task_manager
                    .insert_new_task(task_id, task.get_sender());
                self.time_wheel.push_T_to_time_wheel(task, milliseconds);
                return Ok(task_id);
            } else {
                return Err(Box::new(SchedulerError(
                    DEFAULT_ERROR_CODE_1001.to_string(),
                )));
            }
        } else {
            return Err(Box::new(SchedulerError(
                DEFAULT_ERROR_CODE_1000.to_string(),
            )));
        }
    }

    pub async fn pause_task_by_id(&mut self, task_id: usize) -> bool {
        self.task_manager
            .update_task_status_by_id(task_id, TaskStatus::Pause)
            .await
    }

    pub async fn restart_task_by_id(&mut self, task_id: usize) -> bool {
        self.task_manager
            .update_task_status_by_id(task_id, TaskStatus::Running)
            .await
    }

    pub async fn destory_task_by_id(&mut self, task_id: usize) -> bool {
        self.task_manager
            .update_task_status_by_id(task_id, TaskStatus::Destory)
            .await
    }

    pub async fn update_cron_by_id(
        &mut self,
        task_id: usize,
        cron: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cron_schedule = cron::Schedule::from_str(cron)?;
        if !self
            .task_manager
            .update_cron_by_id(task_id, cron_schedule)
            .await
        {
            return Err(Box::new(SchedulerError(
                DEFAULT_ERROR_CODE_1002.to_string(),
            )));
        }
        Ok(())
    }
}

pub(crate) trait IntoSystem: Sized {
    type System: ITaskHandler + 'static;
    fn to_system(self) -> Self::System;
}

impl<F: SystemParamFunction> IntoSystem for F {
    type System = FunctionSystem<F>;

    fn to_system(self) -> Self::System {
        FunctionSystem {
            func: self,
            _maker: PhantomData,
        }
    }
}

#[derive(Clone)]
pub(crate) struct FunctionSystem<F>
where
    F: SystemParamFunction + 'static,
{
    func: F,
    _maker: PhantomData<()>,
}

impl<F: SystemParamFunction> ITaskHandler for FunctionSystem<F> {
    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        Box::pin(self.func.run())
    }
}

pub(crate) trait SystemParamFunction: Send + Sync + 'static {
    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}

impl<T, Fut> SystemParamFunction for T
where
    T: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        Box::pin(self())
    }
}

#[derive(Debug)]
struct SchedulerError(String);

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = &self.0;
        write!(f, "SchedulerError: {value}")
    }
}

impl Error for SchedulerError {}
