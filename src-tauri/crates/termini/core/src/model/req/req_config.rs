use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveSuggest {
    pub enabled: bool,
    pub count: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveCodeIndex {
    pub ignore: Option<String>,
    pub include_tests: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveIndexDirs {
    #[serde(default)]
    pub dirs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqRebuildIndexDir {
    pub dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveSystem {
    pub system_language: i16,
    #[serde(default)]
    pub local_shell_id: Option<String>,
    pub shell_cpu_percent: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveJobCollect {
    pub exec_timeout_secs: i64,
    pub worker_parallelism: i16,
    pub indicator_value_cache_size: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveJobSshPool {
    pub max_size: i16,
    pub idle_secs: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveAuditPolicy {
    #[serde(default)]
    pub audit_policy: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveJobAlert {
    #[serde(default)]
    pub ai_provide_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveJobDiscover {
    pub parallelism: i16,
    #[serde(default)]
    pub ai_provide_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveRoleModel {
    pub role: i16,
    pub use_cloud: bool,
    #[serde(default)]
    pub dim: Option<i32>,
    #[serde(default)]
    pub ai_provide_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveNotify {
    #[serde(default)]
    pub email: Option<String>,
    pub enabled: bool,
    pub tray_enabled: bool,
    pub webhook_enabled: bool,
    pub webhook_kind: i16,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub webhook_extra: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSaveCloseAction {
    pub close_action: i16,
}
