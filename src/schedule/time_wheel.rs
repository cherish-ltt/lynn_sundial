use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    vec,
};

use chrono::Local;
use tokio::sync::RwLock;

use crate::schedule::{
    config::{
        DEFAULT_HOUR_TIME_WHEEL_SETTING, DEFAULT_MILLISECOND_TIME_WHEEL_SETTING,
        DEFAULT_MINUTE_TIME_WHEEL_SETTING, DEFAULT_SECOND_TIME_WHEEL_SETTING,
    },
    task_actor::{ITaskHandler, Task, TaskPollTrait, TaskSignal, TaskStatus},
};

/// ## 多层时间轮
/// 分4层，分别是：毫秒级、秒级、分钟级、小时级
/// 时间轮每25毫秒tick一次
pub(crate) struct TierTimeWheel {
    millisecond_time_wheel: *mut TimeWheel,
    second_time_wheel: *mut TimeWheel,
    minute_time_wheel: *mut TimeWheel,
    hour_time_wheel: *mut TimeWheel,
    mutex: Mutex<()>,
}

unsafe impl Send for TierTimeWheel {}
unsafe impl Sync for TierTimeWheel {}

impl TierTimeWheel {
    pub(crate) fn new() -> Self {
        Self {
            millisecond_time_wheel: Box::into_raw(Box::new(TimeWheel::new(
                DEFAULT_MILLISECOND_TIME_WHEEL_SETTING.0,
                DEFAULT_MILLISECOND_TIME_WHEEL_SETTING.1,
            ))),
            second_time_wheel: Box::into_raw(Box::new(TimeWheel::new(
                DEFAULT_SECOND_TIME_WHEEL_SETTING.0,
                DEFAULT_SECOND_TIME_WHEEL_SETTING.1,
            ))),
            minute_time_wheel: Box::into_raw(Box::new(TimeWheel::new(
                DEFAULT_MINUTE_TIME_WHEEL_SETTING.0,
                DEFAULT_MINUTE_TIME_WHEEL_SETTING.1,
            ))),
            hour_time_wheel: Box::into_raw(Box::new(TimeWheel::new(
                DEFAULT_HOUR_TIME_WHEEL_SETTING.0,
                DEFAULT_HOUR_TIME_WHEEL_SETTING.1,
            ))),
            mutex: Mutex::new(()),
        }
    }

    pub(crate) fn push_T_to_time_wheel(&self, task: Task, milliseconds: i64) {
        if milliseconds > 1000 {
            let seconds = (milliseconds.abs() / 1000) as u64;
            if seconds > 60 * 60 {
                // 小时级
                self.push_T_to_hour_time_wheel(task, seconds);
            } else if seconds > 60 {
                // 分钟级
                self.push_T_to_minute_time_wheel(task, seconds);
            } else {
                // 秒级
                self.push_T_to_second_time_wheel(task, seconds);
            }
        } else {
            // 毫秒级
            self.push_T_to_millisecond_time_wheel(task, milliseconds as u64);
        }
    }

    fn push_T_to_millisecond_time_wheel(&self, task: Task, milliseconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(millisecond_time_wheel) = unsafe { self.millisecond_time_wheel.as_mut() } {
                let mut target_pointer =
                    millisecond_time_wheel.pointer + (milliseconds / 100) as usize;
                target_pointer = target_pointer % millisecond_time_wheel.slot.len();
                let _ = &millisecond_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    fn push_T_to_second_time_wheel(&self, task: Task, seconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(second_time_wheel) = unsafe { self.second_time_wheel.as_mut() } {
                let mut target_pointer = second_time_wheel.pointer + seconds as usize;
                target_pointer = target_pointer % second_time_wheel.slot.len();
                let _ = &second_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    fn push_T_to_minute_time_wheel(&self, task: Task, seconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(minute_time_wheel) = unsafe { self.minute_time_wheel.as_mut() } {
                let mut target_pointer = minute_time_wheel.pointer + seconds as usize / 60;
                target_pointer = target_pointer % minute_time_wheel.slot.len();
                let _ = &minute_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    fn push_T_to_hour_time_wheel(&self, task: Task, seconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(hour_time_wheel) = unsafe { self.hour_time_wheel.as_mut() } {
                let mut target_pointer = hour_time_wheel.pointer + seconds as usize / 60 / 60;
                if target_pointer >= hour_time_wheel.slot.len() {
                    target_pointer = hour_time_wheel.slot.len() - 1;
                } else {
                    target_pointer = target_pointer % hour_time_wheel.slot.len();
                }
                let _ = &hour_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    pub(crate) async fn tick(
        &self,
        detal: u64,
        notice_list: Arc<RwLock<Option<Vec<(usize, TaskStatus)>>>>,
    ) -> Vec<Arc<Box<dyn ITaskHandler>>> {
        let millisecond_time_wheel = unsafe { self.millisecond_time_wheel.as_mut().unwrap() };
        let second_time_wheel = unsafe { self.second_time_wheel.as_mut().unwrap() };
        let minute_time_wheel = unsafe { self.minute_time_wheel.as_mut().unwrap() };
        let hour_time_wheel = unsafe { self.hour_time_wheel.as_mut().unwrap() };
        millisecond_time_wheel.tick(detal);
        second_time_wheel.tick(detal);
        minute_time_wheel.tick(detal);
        hour_time_wheel.tick(detal);

        let mut return_result = vec![];

        if millisecond_time_wheel.interval_finished() {
            self.check_time_wheel_result(
                millisecond_time_wheel.check(),
                &mut return_result,
                notice_list.clone(),
            )
            .await;
        }
        if second_time_wheel.interval_finished() {
            self.check_time_wheel_result(
                second_time_wheel.check(),
                &mut return_result,
                notice_list.clone(),
            )
            .await;
        }
        if minute_time_wheel.interval_finished() {
            self.check_time_wheel_result(
                minute_time_wheel.check(),
                &mut return_result,
                notice_list.clone(),
            )
            .await;
        }
        if hour_time_wheel.interval_finished() {
            self.check_time_wheel_result(
                hour_time_wheel.check(),
                &mut return_result,
                notice_list.clone(),
            )
            .await;
        }

        return_result
    }

    pub(crate) async fn check_time_wheel_result(
        &self,
        mut time_wheel_result: Vec<Task>,
        return_result: &mut Vec<Arc<Box<dyn ITaskHandler>>>,
        notice_list: Arc<RwLock<Option<Vec<(usize, TaskStatus)>>>>,
    ) {
        let now_time = Local::now();
        loop {
            if let Some(mut t) = time_wheel_result.pop() {
                {
                    let mut mutex = notice_list.write().await;
                    if let Some(vec) = mutex.as_mut() {
                        for index in 0..vec.len() {
                            let (id, status) = &vec[index];
                            if *id == t.get_id() {
                                t.set_status(status.clone());
                                vec.remove(index);
                                break;
                            }
                        }
                    }
                }
                if !t.is_running() {
                    if let Some(next_time) = t.get_next_datetime().await {
                        let time_delta = next_time.signed_duration_since(now_time);
                        let milliseconds = time_delta.num_milliseconds();
                        t.set_target_date_time(next_time).await;
                        self.push_T_to_time_wheel(t, milliseconds);
                    }
                    continue;
                }
                if let Some(target_datetime) = t.get_target_date_time().await {
                    let milliseconds = target_datetime
                        .signed_duration_since(now_time)
                        .num_milliseconds();
                    if milliseconds <= 100 {
                        match t.get_task_order_type() {
                            super::task_actor::TaskOrderType::Order => {
                                let _ =
                                    t.get_task_signal_sender().send(TaskSignal::RunHandle).await;
                            }
                            super::task_actor::TaskOrderType::Disorder => {
                                if let Some(handle) = t.get_handle().await {
                                    return_result.push(handle);
                                }
                            }
                        }
                        if let Some(result) = t.tick_repeat_model().await {
                            if result {
                                if let Some(next_time) = t.get_next_datetime().await {
                                    let time_delta = next_time.signed_duration_since(now_time);
                                    let milliseconds = time_delta.num_milliseconds();
                                    t.set_target_date_time(next_time).await;
                                    self.push_T_to_time_wheel(t, milliseconds);
                                }
                            }
                        }
                    } else {
                        // 降级
                        self.push_T_to_time_wheel(t, milliseconds);
                    }
                }
            } else {
                break;
            }
        }
    }
}

/// ## 时间轮
pub(crate) struct TimeWheel {
    slot: Vec<VecDeque<Task>>,
    pointer: usize,
    interval: u64,
    interval_setting: u64,
}

unsafe impl Send for TimeWheel {}
unsafe impl Sync for TimeWheel {}

impl TimeWheel {
    pub(crate) fn new(slot_len: usize, interval: u64) -> Self {
        let mut slot = Vec::with_capacity(slot_len);
        for _ in 0..slot_len {
            slot.push(VecDeque::new());
        }
        Self {
            slot,
            pointer: 0,
            interval: interval,
            interval_setting: interval,
        }
    }

    pub(crate) fn tick(&mut self, detal: u64) {
        self.interval = self.interval.saturating_sub(detal);
    }

    pub(crate) fn interval_finished(&mut self) -> bool {
        if self.interval <= 0 {
            self.interval = self.interval_setting;
            true
        } else {
            false
        }
    }

    pub(crate) fn check(&mut self) -> Vec<Task> {
        let mut tasks_vec = vec![];
        loop {
            if let Some(t) = self.slot[self.pointer].pop_front() {
                tasks_vec.push(t);
            } else {
                break;
            }
        }
        self.pointer += 1;
        if self.pointer >= self.slot.len() {
            self.pointer = 0;
        }
        tasks_vec
    }
}
