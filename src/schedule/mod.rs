mod config;
mod reactor;
mod task;
mod time_wheel;

#[cfg(feature = "schedule")]
use std::sync::Arc;
use std::{
    collections::VecDeque, default, error::Error, marker::PhantomData, pin::Pin, str::FromStr,
    time::Duration,
};

use chrono::{DateTime, Local, Utc};

use crate::schedule::task::{ITaskHandler, Task};
#[cfg(feature = "schedule")]
use crate::schedule::{
    reactor::TaskReactor,
    time_wheel::{TierTimeWheel, TimeWheel},
};

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
            let seconds = time_delta.num_seconds() as u64;
            let task = Task::new(
                cron_schedule,
                Arc::new(Box::new(handle.to_system())),
                repeat,
                next_time,
            );
            if seconds > 60 {
                if seconds > 60 * 60 {
                    // 小时级
                    self.time_wheel.push_T_to_hour_time_wheel(task, seconds);
                } else {
                    // 分钟级
                    self.time_wheel.push_T_to_minute_time_wheel(task, seconds);
                }
            } else {
                // 秒级
                self.time_wheel.push_T_to_second_time_wheel(task, seconds);
            }
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
