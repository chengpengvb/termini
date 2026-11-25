use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ReqCloudSyncPrepare {
    pub op: i8,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqCloudSyncApply {
    pub op: i8,
    pub password: String,
    pub choices: Vec<CloudSyncChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudSyncChoice {
    pub sync_type: i8,
    pub id: String,
    pub side: i8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReqUserCloudSyncItem {
    pub r#type: i8,
    pub payload: String,
}
