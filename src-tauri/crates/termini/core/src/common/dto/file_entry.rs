use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    #[default]
    File,
    Other,
    Symlink,
    Directory,
    Parent,
}

#[derive(Debug, Clone)]
pub struct PlannedDelete {
    pub path: String,
    pub kind: FileKind,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub hidden: bool,
    pub name: String,
    pub path: String,
    pub kind: FileKind,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub size: Option<u64>,
    pub mtime: Option<u64>,
    pub user: Option<String>,
    pub group: Option<String>,
    pub permissions: Option<u32>,
}
