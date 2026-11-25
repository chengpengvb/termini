use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventAiProject {
    OnChanged(String),
    OnDeleted(String),
}

impl RespEventAiProject {
    pub const ON_CHANGED: i16 = 92;
    pub const ON_DELETED: i16 = 93;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventAiProject::OnChanged(_) => Self::ON_CHANGED,
            RespEventAiProject::OnDeleted(_) => Self::ON_DELETED,
        }
    }

    pub fn project_id(&self) -> &str {
        match self {
            RespEventAiProject::OnChanged(project_id) => project_id,
            RespEventAiProject::OnDeleted(project_id) => project_id,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let project_id = self.project_id().to_string();
        RespSocketContext { r#type: self.to_type(), session_id: project_id.clone(), payload: serde_json::to_value(project_id).ok() }
    }
}
