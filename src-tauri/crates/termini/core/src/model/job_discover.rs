use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum JobDiscoverStatus {
    Queued = 1,    // 排队中，等并发名额
    Running = 2,   // 探索中
    Success = 3,   // 已完成
    Failed = 4,    // 失败
    Cancelled = 5, // 已取消
}

impl JobDiscoverStatus {
    pub fn is_running(self) -> bool {
        matches!(self, JobDiscoverStatus::Queued | JobDiscoverStatus::Running)
    }
}

#[base_target(table = "job_discover", entity_type = 25)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub status: i16,                   // JobDiscoverStatus
    pub chat_id: String,               // 关联 ai_chat.id，用于复用 deck 的对话回显
    pub timestamp: i64,                // 开始时间（毫秒）
    pub job_device_id: String,         // 被探索的任务设备
    pub ended_at: Option<i64>,         // 结束时间（毫秒），进行中为 None
    pub summary: Option<String>,       // 探索总结，Markdown
    pub error: Option<String>,         // 失败原因，仅 Failed 有值
    pub ai_provide_id: Option<String>, // 本次探索用的模型
}

pub type JobDiscover = Model;
