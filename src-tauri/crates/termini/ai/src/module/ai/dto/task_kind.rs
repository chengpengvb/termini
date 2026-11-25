use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    #[default]
    DirectAnswer, // 直接回答：无需工具，一次作答
    MultiStep, // 需要工具或多步推进：进入主 Agent + tools 循环
}

impl TaskKind {
    pub fn as_i16(self) -> i16 {
        match self {
            TaskKind::DirectAnswer => 0,
            TaskKind::MultiStep => 1,
        }
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(TaskKind::DirectAnswer),
            1 => Some(TaskKind::MultiStep),
            _ => None,
        }
    }
}
