use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    vec,
};

use chrono::Local;

use crate::schedule::{
    config::{
        DEFAULT_HOUR_TIME_WHEEL_SETTING, DEFAULT_MILLISECOND_TIME_WHEEL_SETTING,
        DEFAULT_MINUTE_TIME_WHEEL_SETTING, DEFAULT_SECOND_TIME_WHEEL_SETTING,
    },
    task::{ITaskHandler, TaskPollTrait},
};

/// ## 多层时间轮
/// 分4层，分别是：毫秒级、秒级、分钟级、小时级
/// 时间轮每25毫秒tick一次
pub(crate) struct TierTimeWheel<T>
where
    T: TaskPollTrait + 'static,
{
    millisecond_time_wheel: *mut TimeWheel<T>,
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

    pub(crate) fn push_T_to_time_wheel(&self, task: T, milliseconds: i64) {
        if milliseconds > 1000 {
            let seconds = (milliseconds.abs() / 1000) as u64;
            // println!("push-seconds:{}", seconds);
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

    fn push_T_to_millisecond_time_wheel(&self, task: T, milliseconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(millisecond_time_wheel) = unsafe { self.millisecond_time_wheel.as_mut() } {
                let mut target_pointer =
                    millisecond_time_wheel.pointer + (milliseconds / 100) as usize;
                target_pointer = target_pointer % millisecond_time_wheel.slot.len();
                let _ = &millisecond_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    fn push_T_to_second_time_wheel(&self, task: T, seconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(second_time_wheel) = unsafe { self.second_time_wheel.as_mut() } {
                let mut target_pointer = second_time_wheel.pointer + seconds as usize;
                target_pointer = target_pointer % second_time_wheel.slot.len();
                let _ = &second_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    fn push_T_to_minute_time_wheel(&self, task: T, seconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(minute_time_wheel) = unsafe { self.minute_time_wheel.as_mut() } {
                let mut target_pointer = minute_time_wheel.pointer + seconds as usize / 60;
                target_pointer = target_pointer % minute_time_wheel.slot.len();
                let _ = &minute_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    fn push_T_to_hour_time_wheel(&self, task: T, seconds: u64) {
        if let Ok(_mutex) = self.mutex.lock() {
            if let Some(hour_time_wheel) = unsafe { self.hour_time_wheel.as_mut() } {
                let mut target_pointer = hour_time_wheel.pointer + seconds as usize / 60 / 60;
                target_pointer = target_pointer % hour_time_wheel.slot.len();
                let _ = &hour_time_wheel.slot[target_pointer as usize].push_back(task);
            }
        }
    }

    pub(crate) async fn tick(&self, detal: u64) -> Vec<Arc<Box<dyn ITaskHandler>>> {
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
            self.check_time_wheel_result(millisecond_time_wheel.check(), &mut return_result);
        }
        if second_time_wheel.interval_finished() {
            self.check_time_wheel_result(second_time_wheel.check(), &mut return_result);
        }
        if minute_time_wheel.interval_finished() {
            self.check_time_wheel_result(minute_time_wheel.check(), &mut return_result);
        }
        if hour_time_wheel.interval_finished() {
            self.check_time_wheel_result(hour_time_wheel.check(), &mut return_result);
        }

        return_result
    }

    pub(crate) fn check_time_wheel_result(
        &self,
        mut time_wheel_result: Vec<T>,
        return_result: &mut Vec<Arc<Box<dyn ITaskHandler>>>,
    ) {
        let now_time = Local::now();
        loop {
            if let Some(mut t) = time_wheel_result.pop() {
                let milliseconds = t
                    .get_target_date_time()
                    .signed_duration_since(now_time)
                    .num_milliseconds();
                if milliseconds <= 100 {
                    return_result.push(t.get_handle());
                    if t.tick_repeat_model() {
                        if let Some(next_time) = t.get_next_datetime() {
                            let time_delta = next_time.signed_duration_since(now_time);
                            let milliseconds = time_delta.num_milliseconds();
                            t.set_target_date_time(next_time);
                            self.push_T_to_time_wheel(t, milliseconds);
                        }
                    }
                } else {
                    // 降级
                    self.push_T_to_time_wheel(t, milliseconds);
                }
            } else {
                break;
            }
        }
    }
}

/// ## 时间轮
pub(crate) struct TimeWheel<T>
where
    T: TaskPollTrait + 'static,
{
    slot: Vec<VecDeque<T>>,
    pointer: usize,
    interval: u64,
    interval_setting: u64,
}

unsafe impl<T> Send for TimeWheel<T> where T: TaskPollTrait + 'static {}
unsafe impl<T> Sync for TimeWheel<T> where T: TaskPollTrait + 'static {}

impl<T> TimeWheel<T>
where
    T: TaskPollTrait + 'static,
{
    pub(crate) fn new(slot_len: usize, interval: u64) -> Self {
        let mut slot = Vec::with_capacity(slot_len);
        for _ in 0..slot_len {
            slot.push(VecDeque::<T>::new());
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
