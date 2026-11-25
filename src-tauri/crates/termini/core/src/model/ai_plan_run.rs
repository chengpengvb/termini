use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[repr(i16)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AiPlanRunStatus {
    Running = 1,
    Success = 2,
    Failed = 3,
}

#[base_target(table = "ai_plan_run", entity_type = 21, notify = 23)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Model {
    pub task_id: String,
    pub chat_id: String,
    pub status: i16,
    pub started_at: i64,
    pub ended_at: i64,
    pub summary: Option<String>,
    pub error: Option<String>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            task_id: String::new(),
            chat_id: String::new(),
            status: AiPlanRunStatus::Running as i16,
            started_at: 0,
            ended_at: 0,
            summary: None,
            error: None,
        }
    }
}

pub type AiPlanRun = Model;
