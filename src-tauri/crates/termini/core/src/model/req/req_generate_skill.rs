use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqGenerateSkill {
    pub task_id: String,
    pub ai_provide_id: String,
    pub chat_id: Option<String>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub content: Option<String>,

    #[serde(default)]
    pub key_points: Option<String>,

    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqCancelGenerateSkill {
    pub task_id: String,
}
