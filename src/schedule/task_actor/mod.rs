use std::{pin::Pin, sync::Arc};

use crate::schedule::{RepeatModel, config::DEFAULT_CHANNEL_SIZE};
use chrono::{DateTime, Local};
use cron::Schedule;
use tokio::sync::mpsc::{Receiver, Sender, channel};

pub(crate) trait TaskPollTrait {
    async fn get_target_date_time(&mut self) -> Option<DateTime<Local>>;
    fn get_task_order_type(&mut self) -> TaskOrderType;
    fn get_task_signal_sender(&mut self) -> Sender<TaskSignal>;
    async fn get_handle(&mut self) -> Option<Arc<Box<dyn ITaskHandler>>>;
    async fn tick_repeat_model(&mut self) -> Option<bool>;
    async fn get_next_datetime(&self) -> Option<DateTime<Local>>;
    async fn set_target_date_time(&mut self, target_datetime: DateTime<Local>);
}
pub(crate) trait TaskActorTrait {
    fn get_handle(&self) -> Arc<Box<dyn ITaskHandler>>;
    fn get_next_datetime(&self) -> Option<DateTime<Local>>;
    fn tick_repeat_model(&mut self) -> bool;
    fn set_target_date_time(&mut self, target_datetime: DateTime<Local>);
    fn get_target_date_time(&mut self) -> DateTime<Local>;
}

/// 任务信号
#[derive(Clone)]
pub(crate) enum TaskSignal {
    /// 获取handle
    GetHandle(Sender<Arc<Box<dyn ITaskHandler>>>),
    /// 运行一次handle
    RunHandle,
    /// 获取下次运行时间
    GetNextDatetime(Sender<Option<DateTime<Local>>>),
    /// 获取是否需要重复运行
    TickRepeatModel(Sender<bool>),
    /// 设置目标时间
    SetTargetDateTime(DateTime<Local>),
    /// 获取目标时间
    GetTargetDateTime(Sender<DateTime<Local>>),
    /// 销毁
    Destory,
    /// 暂停
    Pause(Sender<TaskActor>),
    /// 更新cron
    UpdateCron(Schedule),
}

pub(crate) enum TaskOrderType {
    /// 有序
    Order,
    /// 无序
    Disorder,
}

#[derive(Clone)]
pub(crate) enum TaskStatus {
    /// 暂停,挂起
    Pause,
    /// 销毁
    Destory,
    /// 运行中
    Running,
}

pub(crate) struct Task {
    task_signal_sender: Sender<TaskSignal>,
    task_order_type: TaskOrderType,
    task_status: TaskStatus,
    task_id: usize,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    pub(crate) fn new(
        cron_schedule: cron::Schedule,
        handle: Arc<Box<dyn ITaskHandler>>,
        repeat_model: RepeatModel,
        target_datetime: DateTime<Local>,
        task_order_type: TaskOrderType,
        task_id: usize,
    ) -> Self {
        Self {
            task_signal_sender: TaskActor::new(
                cron_schedule,
                handle,
                repeat_model,
                target_datetime,
            ),
            task_order_type,
            task_status: TaskStatus::Running,
            task_id,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        match self.task_status {
            TaskStatus::Pause => false,
            TaskStatus::Destory => false,
            TaskStatus::Running => true,
        }
    }

    pub(crate) fn set_status(&mut self, status: TaskStatus) {
        self.task_status = status;
    }

    pub(crate) fn get_id(&self) -> usize {
        self.task_id
    }

    pub(crate) fn get_sender(&self) -> Sender<TaskSignal> {
        self.task_signal_sender.clone()
    }
}

impl TaskPollTrait for Task {
    async fn get_target_date_time(&mut self) -> Option<DateTime<Local>> {
        let (tx, mut rx) = channel(1);
        let _ = self
            .task_signal_sender
            .send(TaskSignal::GetTargetDateTime(tx))
            .await;
        if let Some(datetime) = rx.recv().await {
            return Some(datetime);
        } else {
            None
        }
    }

    fn get_task_order_type(&mut self) -> TaskOrderType {
        match self.task_order_type {
            TaskOrderType::Order => TaskOrderType::Order,
            TaskOrderType::Disorder => TaskOrderType::Disorder,
        }
    }

    fn get_task_signal_sender(&mut self) -> Sender<TaskSignal> {
        self.task_signal_sender.clone()
    }

    async fn get_handle(&mut self) -> Option<Arc<Box<dyn ITaskHandler>>> {
        let (tx, mut rx) = channel(1);
        let _ = self
            .task_signal_sender
            .send(TaskSignal::GetHandle(tx))
            .await;
        if let Some(handle) = rx.recv().await {
            return Some(handle);
        } else {
            None
        }
    }

    async fn tick_repeat_model(&mut self) -> Option<bool> {
        let (tx, mut rx) = channel(1);
        let _ = self
            .task_signal_sender
            .send(TaskSignal::TickRepeatModel(tx))
            .await;
        if let Some(result) = rx.recv().await {
            return Some(result);
        } else {
            None
        }
    }

    async fn get_next_datetime(&self) -> Option<DateTime<Local>> {
        let (tx, mut rx) = channel(1);
        let _ = self
            .task_signal_sender
            .send(TaskSignal::GetNextDatetime(tx))
            .await;
        if let Some(result) = rx.recv().await {
            return result;
        } else {
            None
        }
    }

    async fn set_target_date_time(&mut self, target_datetime: DateTime<Local>) {
        let _ = self
            .task_signal_sender
            .send(TaskSignal::SetTargetDateTime(target_datetime))
            .await;
    }
}

/// ## 任务actor
pub(crate) struct TaskActor {
    cron_schedule: cron::Schedule,
    handle: Arc<Box<dyn ITaskHandler>>,
    repeat_model: RepeatModel,
    target_datetime: DateTime<Local>,
    receiver: Receiver<TaskSignal>,
}

impl TaskActor {
    fn new(
        cron_schedule: cron::Schedule,
        handle: Arc<Box<dyn ITaskHandler>>,
        repeat_model: RepeatModel,
        target_datetime: DateTime<Local>,
    ) -> Sender<TaskSignal> {
        let (tx, rx) = channel::<TaskSignal>(DEFAULT_CHANNEL_SIZE);
        let task_actor = Self {
            cron_schedule,
            handle,
            repeat_model,
            target_datetime,
            receiver: rx,
        };
        task_actor.start_actor();
        tx
    }

    pub(crate) fn start_actor(self) {
        let mut task_actor = self;
        tokio::spawn(async move {
            loop {
                if let Some(task_signal) = task_actor.get_signal().await {
                    match task_signal {
                        TaskSignal::GetHandle(sender) => {
                            let _ = sender.send(task_actor.get_handle()).await;
                        }
                        TaskSignal::RunHandle => {
                            let _ = task_actor.get_handle().run().await;
                        }
                        TaskSignal::GetNextDatetime(sender) => {
                            let _ = sender.send(task_actor.get_next_datetime()).await;
                        }
                        TaskSignal::TickRepeatModel(sender) => {
                            let _ = sender.send(task_actor.tick_repeat_model()).await;
                        }
                        TaskSignal::SetTargetDateTime(date_time) => {
                            task_actor.set_target_date_time(date_time);
                        }
                        TaskSignal::GetTargetDateTime(sender) => {
                            let _ = sender.send(task_actor.get_target_date_time()).await;
                        }
                        TaskSignal::Destory => break,
                        TaskSignal::Pause(sender) => {
                            sender.send(task_actor).await;
                            break;
                        }
                        TaskSignal::UpdateCron(schedule) => {
                            task_actor.cron_schedule = schedule;
                            if let Some(datetime) = task_actor.cron_schedule.upcoming(Local).next()
                            {
                                task_actor.target_datetime = datetime;
                            }
                        }
                    }
                }
            }
        });
    }

    async fn get_signal(&mut self) -> Option<TaskSignal> {
        self.receiver.recv().await
    }
}

impl TaskActorTrait for TaskActor {
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
