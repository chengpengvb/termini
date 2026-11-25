use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReqGroupSet {
    pub group_id: String,
    pub device_id: String,
}
