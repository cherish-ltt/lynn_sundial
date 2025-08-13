mod config;
mod reactor;
mod task;
mod time_wheel;

#[cfg(feature = "schedule")]
use std::sync::Arc;
use std::{marker::PhantomData, pin::Pin, str::FromStr};

use chrono::Local;

use crate::schedule::task::{ITaskHandler, Task};
#[cfg(feature = "schedule")]
use crate::schedule::{reactor::TaskReactor, time_wheel::TierTimeWheel};

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
    pub(crate) time_wheel: Arc<TierTimeWheel<Task>>,
    pub(crate) task_reactor: TaskReactor,
}

impl Scheduler {
    pub fn new() -> Self {
        let time_wheel = Arc::new(TierTimeWheel::<Task>::new());
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

    /// #### 添加定时任务
    /// 注意：
    /// - 所有为RepeatModel::Repetition的同一定时任务（如：任务A），上一次任务运行（任务A.run）和下一次任务运行（任务A.run）是可以并行的
    /// - 如需同一任务在上一次任务运行还未结束时，不允许下一次任务直接运行，请使用api：`push_sync_task`(还未支持)
    pub fn push_task(
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
