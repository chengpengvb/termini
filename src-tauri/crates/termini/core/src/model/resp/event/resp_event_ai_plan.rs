use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventAiPlan {
    OnChanged(String), //task_id：任务或其运行记录发生变化
}

impl RespEventAiPlan {
    pub const ON_CHANGED: i16 = 104;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventAiPlan::OnChanged(_) => Self::ON_CHANGED,
        }
    }

    pub fn task_id(&self) -> &str {
        match self {
            RespEventAiPlan::OnChanged(task_id) => task_id,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let task_id = self.task_id().to_string();
        RespSocketContext { r#type: self.to_type(), session_id: task_id.clone(), payload: serde_json::to_value(task_id).ok() }
    }
}
