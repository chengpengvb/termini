use sea_orm::JsonValue;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use crate::model::job_indicator::JobIndicator;
use crate::model::job_indicator_value::JobIndicatorValue;

pub enum AlertStatus {
    Firing = 1, // 告警中
    Normal = 2, // 恢复
}

#[derive(Clone, Copy)]
pub enum FireType {
    CollectFailed = 1, // 采集失败
    Rule = 2,          // 规则命中
}

#[derive(Clone, Copy)]
pub enum ResolveType {
    Rule = 1,     // 规则恢复
    AlertOff = 2, // 关闭告警
}

#[base_target(table = "job_indicator_alert", entity_type = 35)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub status: i16,                    // AlertStatus
    pub alert_at: i64,                  // 告警时间（Unix 毫秒）
    pub alert_rule_op: i16,             // 告警规则
    pub alert_value: String,            // 告警值（采集失败时可为空）
    pub fire_type: Option<i16>,         // FireType；旧数据可能为 None
    pub job_device_id: String,          // 设备 ID
    pub alert_threshold: String,        // 告警阈值
    pub job_indicator_id: String,       // 指标 ID
    pub resolved_at: Option<i64>,       // 恢复时间（Unix 毫秒）
    pub ai_summary: Option<String>,     // AI 诊断摘要，可空
    pub ai_chat_id: Option<String>,     // AI 诊断会话 ID，可空
    pub resolved_type: Option<i16>,     // ResolveType；未恢复为 None
    pub resolved_value: Option<String>, // 恢复值；关闭告警时为 None
    pub job_indicator_value: JsonValue, // JSON: 触发时 JobIndicatorValue 快照

    #[sea_orm(ignore)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_indicator: Option<JobIndicator>, // 查询填充，不落库
}

impl Model {
    pub fn get_job_indicator_value(&self) -> anyhow::Result<Option<JobIndicatorValue>> {
        if self.job_indicator_value.is_null() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_value(self.job_indicator_value.clone())?))
    }

    pub fn set_job_indicator_value(&mut self, value: Option<JobIndicatorValue>) -> anyhow::Result<()> {
        self.job_indicator_value = match value {
            Some(v) => serde_json::to_value(v)?,
            None => JsonValue::Null,
        };
        Ok(())
    }
}

pub type JobIndicatorAlert = Model;
