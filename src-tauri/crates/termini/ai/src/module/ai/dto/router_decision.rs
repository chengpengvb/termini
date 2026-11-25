use serde::{Deserialize, Serialize};
use crate::module::ai::dto::TaskKind;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RouterDecision {
    #[serde(default)]
    pub task_kind: TaskKind,

    #[serde(default)]
    pub goal: Option<String>,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub language: Option<String>,
}
