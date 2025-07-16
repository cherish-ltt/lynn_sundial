use std::{pin::Pin, sync::Arc};

use chrono::{DateTime, Local};

use crate::schedule::RepeatModel;

pub(crate) trait TaskPollTrait {
    fn get_handle(&self) -> Arc<Box<dyn ITaskHandler>>;
    fn get_next_datetime(&self) -> Option<DateTime<Local>>;
    fn tick_repeat_model(&mut self) -> bool;
    fn set_target_date_time(&mut self, target_datetime: DateTime<Local>);
    fn get_target_date_time(&mut self) -> DateTime<Local>;
}

/// ## 任务
pub(crate) struct Task {
    cron_schedule: cron::Schedule,
    handle: Arc<Box<dyn ITaskHandler>>,
    repeat_model: RepeatModel,
    target_datetime: DateTime<Local>,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    pub(crate) fn new(
        cron_schedule: cron::Schedule,
        handle: Arc<Box<dyn ITaskHandler>>,
        repeat_model: RepeatModel,
        target_datetime: DateTime<Local>,
    ) -> Self {
        Self {
            cron_schedule,
            handle,
            repeat_model,
            target_datetime,
        }
    }
}

impl TaskPollTrait for Task {
    fn get_handle(&self) -> Arc<Box<dyn ITaskHandler>> {
        self.handle.clone()
    }

    fn get_next_datetime(&self) -> Option<DateTime<Local>> {
        self.cron_schedule.upcoming(Local).next()
    }

    fn tick_repeat_model(&mut self) -> bool {
        match &mut self.repeat_model {
            RepeatModel::Once => false,
            RepeatModel::Repetition => true,
            RepeatModel::Times(times) => {
                *times -= 1;
                if *times > 0 { true } else { false }
            }
        }
    }

    fn set_target_date_time(&mut self, target_datetime: DateTime<Local>) {
        self.target_datetime = target_datetime;
    }

    fn get_target_date_time(&mut self) -> DateTime<Local> {
        self.target_datetime.clone()
    }
}

pub(crate) trait ITaskHandler: Send + Sync + 'static {
    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}
