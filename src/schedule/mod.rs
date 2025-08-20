mod config;
mod reactor;
mod task_actor;
mod time_wheel;

use crate::schedule::task_actor::{ITaskHandler, Task, TaskOrderType};
use crate::schedule::{reactor::TaskReactor, time_wheel::TierTimeWheel};
use chrono::Local;
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
}

impl Scheduler {
    pub fn new() -> Self {
        let time_wheel = Arc::new(TierTimeWheel::new());
        let mut task_reactor = TaskReactor::new();
        task_reactor.start(time_wheel.clone());

        Self {
            time_wheel,
            task_reactor,
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
    ) -> Result<(), cron::error::Error> {
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
    ) -> Result<(), cron::error::Error> {
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
            );
            self.time_wheel.push_T_to_time_wheel(task, milliseconds);
        }
        Ok(())
    }

    /// #### 新增无序定时任务
    /// 注意：
    /// - 所有为`RepeatModel::Repetition/::Times(>0)`的同一定时任务，无需关注上一次任务是否结束就允许运行新的任务（如：任务A，无需关注上一次任务A是否结束，就允许新任务A运行的定时任务）
    pub fn push_disorder_task(
        &mut self,
        cron: &str,
        handle: impl IntoSystem,
        repeat: RepeatModel,
    ) -> Result<(), cron::error::Error> {
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
            );
            self.time_wheel.push_T_to_time_wheel(task, milliseconds);
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
