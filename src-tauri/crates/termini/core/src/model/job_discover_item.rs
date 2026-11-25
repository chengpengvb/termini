use sea_orm::JsonValue;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum JobDiscoverItemStatus {
    Candidate = 1,
    Accepted = 2,
}

#[base_target(table = "job_discover_item", entity_type = 34, notify = 25)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub status: i16,
    pub protocol: i16,
    pub timestamp: i64,
    pub arg: JsonValue,
    pub parser_type: i16,
    pub display_unit: i16,
    pub interval_secs: i64,
    pub parse_expr: String,
    pub alert_rule_op: i16,
    pub discover_id: String,
    pub job_device_id: String,
    pub reason: Option<String>,
    pub transform_expr: String,
    pub alert_threshold: String,
    pub range_min: Option<f64>,
    pub range_max: Option<f64>,
    pub sample_value: Option<String>,
    pub job_indicator_id: Option<String>,
}

pub type JobDiscoverItem = Model;
