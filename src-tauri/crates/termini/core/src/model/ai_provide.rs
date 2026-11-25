use serde_json::Value;
use serde::{Deserialize, Serialize};
use termini_macros::{base_target, Redact};

pub const KIND_CHAT: i16 = 0;
pub const KIND_EMBEDDING: i16 = 1;
pub const KIND_IMAGE_GEN: i16 = 2;
pub const KIND_VIDEO: i16 = 3;

#[base_target(table = "ai_provide", entity_type = 20)]
#[derive(Clone, Default, Serialize, Deserialize, Redact)]
pub struct Model {
    pub url: String,
    pub model: String,
    #[redact]
    pub key: Option<String>,
    pub provider: Option<i16>,
    pub max_token: Option<u32>,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub kind: i16,
    #[sea_orm(ignore)]
    #[serde(default)]
    pub source: i8,
    #[sea_orm(ignore)]
    #[serde(default)]
    pub hidden: bool,
    #[sea_orm(ignore)]
    #[serde(default)]
    pub params: Option<Value>,
    #[sea_orm(ignore)]
    #[serde(default)]
    pub input_billing_rate: Option<f64>,
    #[sea_orm(ignore)]
    #[serde(default)]
    pub output_billing_rate: Option<f64>,
    #[sea_orm(ignore)]
    #[serde(default)]
    pub thinking_levels: Option<Value>,
}

impl Model {
    pub fn thinking_level_params(&self, level: Option<&str>) -> Option<serde_json::Map<String, Value>> {
        let levels = self.thinking_levels.as_ref()?.get("levels")?.as_object()?;
        let name = level.filter(|name| !name.is_empty()).or_else(|| self.thinking_levels.as_ref()?.get("default")?.as_str())?;
        levels.get(name)?.as_object().cloned()
    }
}

pub type AiProvide = Model;
