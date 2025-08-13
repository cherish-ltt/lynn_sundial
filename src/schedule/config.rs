pub(crate) const DEFAULT_MILLISECOND_TIME_WHEEL_SETTING: (usize, u64) = (10, 100);
pub(crate) const DEFAULT_SECOND_TIME_WHEEL_SETTING: (usize, u64) = (60, 1000);
pub(crate) const DEFAULT_MINUTE_TIME_WHEEL_SETTING: (usize, u64) = (60, 60 * 1000);
pub(crate) const DEFAULT_HOUR_TIME_WHEEL_SETTING: (usize, u64) = (24, 1 * 60 * 60 * 1000);
/// 默认的tick间隔 25毫秒
pub(crate) const DEFAULT_TICK_TIME: u64 = 25;
/// 默认的任务线程数量
pub(crate) const DEFAULT_TASK_POOL_SIZE: usize = if usize::MAX as u128 == u64::MAX as u128 {
    64
} else {
    32
};
