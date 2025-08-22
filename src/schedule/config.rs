/// 默认毫秒时间轮配置，分10层，单位100ms
pub(crate) const DEFAULT_MILLISECOND_TIME_WHEEL_SETTING: (usize, u64) = (10, 100);
/// 默认秒时间轮配置，分60层，单位1s
pub(crate) const DEFAULT_SECOND_TIME_WHEEL_SETTING: (usize, u64) = (60, 1000);
/// 默认分钟时间轮配置，分60层，单位1min
pub(crate) const DEFAULT_MINUTE_TIME_WHEEL_SETTING: (usize, u64) = (60, 60 * 1000);
/// 默认小时时间轮配置，分24层，单位1小时
pub(crate) const DEFAULT_HOUR_TIME_WHEEL_SETTING: (usize, u64) = (24, 1 * 60 * 60 * 1000);
/// 默认的tick间隔 25毫秒
pub(crate) const DEFAULT_TICK_TIME: u64 = 25;
/// 默认的副reactor线程数量（tokio线程，非真实thread）
pub(crate) const DEFAULT_TASK_POOL_SIZE: usize = 32;
/// 默认的channel大小
pub(crate) const DEFAULT_CHANNEL_SIZE: usize = 8;

// Error
/// 没有可以分配的taskid了
pub(crate) const DEFAULT_ERROR_CODE_1000: &str =
    "ERR_CODE(1000),The task pool no longer has IDs to allocate";
/// cron错误
pub(crate) const DEFAULT_ERROR_CODE_1001: &str = "ERR_CODE(1001),Cron operation failed";
/// 更新cron到task出现未知原因错误
pub(crate) const DEFAULT_ERROR_CODE_1002: &str =
    "ERR_CODE(1002),Updating cron failed for unknown reason";
