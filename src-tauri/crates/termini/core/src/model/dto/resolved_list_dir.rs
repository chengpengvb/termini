use crate::common::dto::file_entry::FileEntry;

#[derive(Debug, Clone)]
pub struct ResolvedListDir {
    pub effective_path: String,
    pub entries: Vec<FileEntry>,
}
