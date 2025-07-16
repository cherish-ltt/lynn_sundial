use core::task;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
    vec,
};

use chrono::Local;
use tokio::{
    select,
    time::{Interval, interval, sleep},
};

use crate::schedule::{
    config::{
        DEFAULT_HOUR_TIME_WHEEL_SETTING, DEFAULT_MINUTE_TIME_WHEEL_SETTING,
        DEFAULT_SECOND_TIME_WHEEL_SETTING,
    },
    task::{ITaskHandler, Task, TaskPollTrait},
};

/// ## 多层时间轮
/// 分3层，分别是：秒级、分钟级、小时级
pub(crate) struct TierTimeWheel<T>
where
    T: TaskPollTrait + 'static,
{
    second_time_wheel: *mut TimeWheel<T>,
    minute_time_wheel: *mut TimeWheel<T>,
    hour_time_wheel: *mut TimeWheel<T>,
    mutex: Mutex<()>,
}

unsafe impl<T> Send for TierTimeWheel<T> where T: TaskPollTrait + 'static {}
unsafe impl<T> Sync for TierTimeWheel<T> where T: TaskPollTrait + 'static {}

impl<T> TierTimeWheel<T>
where
    T: TaskPollTrait,
{
    pub(crate) fn new() -> Self {
        Self {
            second_time_wheel: Box::into_raw(Box::new(TimeWheel::new(
                DEFAULT_SECOND_TIME_WHEEL_SETTING.0,
                Duration::from_secs(DEFAULT_SECOND_TIME_WHEEL_SETTING.1),
            ))),
            minute_time_wheel: Box::into_raw(Box::new(TimeWheel::new(
                DEFAULT_MINUTE_TIME_WHEEL_SETTING.0,
                Duration::from_secs(DEFAULT_MINUTE_TIME_WHEEL_SETTING.1),
            ))),
            hour_time_wheel: Box::into_raw(Box::new(TimeWheel::new(
                DEFAULT_HOUR_TIME_WHEEL_SETTING.0,
                Duration::from_secs(DEFAULT_HOUR_TIME_WHEEL_SETTING.1),
            ))),
            mutex: Mutex::new(()),
        }
    }

    pub(crate) fn push_T_to_second_time_wheel(&self, task: T, seconds: u64) {
        self.mutex.lock();
        if let Some(second_time_wheel) = unsafe { self.second_time_wheel.as_mut() } {
            let mut target_pointer = second_time_wheel.pointer + seconds as usize;
            target_pointer = target_pointer % second_time_wheel.slot.len();
            &second_time_wheel.slot[target_pointer as usize].push_back(task);
        }
    }

    pub(crate) fn push_T_to_minute_time_wheel(&self, task: T, seconds: u64) {
        self.mutex.lock();
        if let Some(minute_time_wheel) = unsafe { self.minute_time_wheel.as_mut() } {
            let mut target_pointer = minute_time_wheel.pointer + seconds as usize / 60;
            target_pointer = target_pointer % minute_time_wheel.slot.len();
            &minute_time_wheel.slot[target_pointer as usize].push_back(task);
        }
    }

    pub(crate) fn push_T_to_hour_time_wheel(&self, task: T, seconds: u64) {
        self.mutex.lock();
        if let Some(hour_time_wheel) = unsafe { self.hour_time_wheel.as_mut() } {
            let mut target_pointer = hour_time_wheel.pointer + seconds as usize / 60 / 60;
            target_pointer = target_pointer % hour_time_wheel.slot.len();
            &hour_time_wheel.slot[target_pointer as usize].push_back(task);
        }
    }

    pub(crate) async fn tick(&self) -> Vec<Arc<Box<dyn ITaskHandler>>> {
        let second_time_wheel = unsafe { self.second_time_wheel.as_mut().unwrap() };
        let minute_time_wheel = unsafe { self.minute_time_wheel.as_mut().unwrap() };
        let hour_time_wheel = unsafe { self.hour_time_wheel.as_mut().unwrap() };
        let mut result;

        tokio::select! {
            _ = sleep(second_time_wheel.slot_interval) =>{
                result = second_time_wheel.check();
            }
            _ = sleep(minute_time_wheel.slot_interval)=>{
                result = minute_time_wheel.check();
            }
            _ = sleep(hour_time_wheel.slot_interval)=>{
                result = hour_time_wheel.check();
            }
        }
        let mut return_result = vec![];
        loop {
            if let Some(mut t) = result.pop() {
                let now_time = Local::now();
                let seconds = t
                    .get_target_date_time()
                    .signed_duration_since(now_time)
                    .num_seconds();
                if seconds <= 0 {
                    return_result.push(t.get_handle());
                    if t.tick_repeat_model() {
                        if let Some(next_time) = t.get_next_datetime() {
                            let time_delta = next_time.signed_duration_since(now_time);
                            t.set_target_date_time(next_time);
                            let seconds = time_delta.num_seconds().abs() as u64;
                            if seconds > 60 {
                                if seconds > 60 * 60 {
                                    // 小时级
                                    self.push_T_to_hour_time_wheel(t, seconds);
                                } else {
                                    // 分钟级
                                    self.push_T_to_minute_time_wheel(t, seconds);
                                }
                            } else {
                                // 秒级
                                self.push_T_to_second_time_wheel(t, seconds);
                            }
                        }
                    }
                } else {
                    // 降级
                    if seconds > 60 {
                        if seconds > 60 * 60 {
                            // 小时级
                            self.push_T_to_hour_time_wheel(t, seconds.abs() as u64);
                        } else {
                            // 分钟级
                            self.push_T_to_minute_time_wheel(t, seconds.abs() as u64);
                        }
                    } else {
                        // 秒级
                        self.push_T_to_second_time_wheel(t, seconds.abs() as u64);
                    }
                }
            } else {
                break;
            }
        }
        return_result
    }
}

/// ## 时间轮
pub(crate) struct TimeWheel<T>
where
    T: TaskPollTrait + 'static,
{
    slot: Vec<VecDeque<T>>,
    pointer: usize,
    pub(crate) slot_interval: Duration,
}

unsafe impl<T> Send for TimeWheel<T> where T: TaskPollTrait + 'static {}
unsafe impl<T> Sync for TimeWheel<T> where T: TaskPollTrait + 'static {}

impl<T> TimeWheel<T>
where
    T: TaskPollTrait + 'static,
{
    pub(crate) fn new(slot_len: usize, duration: Duration) -> Self {
        let mut slot = Vec::with_capacity(slot_len);
        for _ in 0..slot_len {
            slot.push(VecDeque::<T>::new());
        }
        Self {
            slot,
            pointer: 0,
            slot_interval: duration,
        }
    }

    pub(crate) fn check(&mut self) -> Vec<T> {
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
