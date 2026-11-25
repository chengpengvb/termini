use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct RespCloudSyncConflict {
    pub sync_type: i8,
    pub id: String,
    pub local: Value,
    pub cloud: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct RespCloudSyncPrepare {
    pub conflicts: Vec<RespCloudSyncConflict>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespCloudSyncStatus {
    pub has_backup: bool,
    pub update_time: i64,
}
