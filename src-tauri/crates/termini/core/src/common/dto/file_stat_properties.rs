use serde::Serialize;
use crate::common::dto::file_entry::FileEntry;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirStatProperties {
    pub total_size: u64,
    pub file_count: u64,
    pub dir_count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStatProperties {
    pub path: String,
    pub entry: FileEntry,
    pub dir: Option<DirStatProperties>,
}
