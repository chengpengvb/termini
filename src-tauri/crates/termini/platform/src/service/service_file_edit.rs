use anyhow::Context;
use std::path::PathBuf;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::resp::resp_file_edit::RespFileEditRead;
use crate::model::req::req_file_edit::{ReqFileEditRead, ReqFileEditWrite};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[base_app(sort = 122)]
#[derive(Clone, Service)]
pub struct FileEditService {}

#[async_trait]
impl BaseApp for FileEditService {
    fn name(&self) -> &'static str {
        "FileEditService"
    }
}

impl FileEditService {
    pub async fn read(&self, req: ReqFileEditRead) -> anyhow::Result<RespFileEditRead> {
        let path = Self::resolve_path(&req.path)?;
        let meta = Self::resolve_file_meta(&path).await?;
        if meta.len() > MAX_FILE_BYTES {
            anyhow::bail!("file exceeds {} MB and cannot be opened: {}", MAX_FILE_BYTES / 1024 / 1024, path.display());
        }
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => anyhow::bail!("file is not UTF-8 text and cannot be opened: {}", path.display()),
            Err(error) => return Err(anyhow::anyhow!("{}: {}", path.display(), error)),
        };
        let name = path.file_name().map(|value| value.to_string_lossy().to_string()).unwrap_or_else(|| req.path.clone());
        Ok(RespFileEditRead { path: req.path, name, content })
    }

    pub async fn write(&self, req: ReqFileEditWrite) -> anyhow::Result<()> {
        let path = Self::resolve_path(&req.path)?;
        if req.create && tokio::fs::metadata(&path).await.is_err() {
            if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
                tokio::fs::create_dir_all(parent).await.with_context(|| format!("create dir {:?}", parent))?;
            }
            return tokio::fs::write(&path, &req.content).await.with_context(|| format!("write {:?}", path));
        }
        let meta = Self::resolve_file_meta(&path).await?;
        Self::write_atomic(&path, &req.content, meta.permissions()).await
    }

    fn resolve_path(path: &str) -> anyhow::Result<PathBuf> {
        if path.trim().is_empty() {
            anyhow::bail!("file path is empty");
        }
        let raw = PathBuf::from(path);
        if !raw.is_absolute() {
            anyhow::bail!("path must be absolute: {}", path);
        }
        Ok(raw)
    }

    async fn resolve_file_meta(path: &PathBuf) -> anyhow::Result<std::fs::Metadata> {
        let meta = match tokio::fs::metadata(path).await {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => anyhow::bail!("file not found: {}", path.display()),
            Err(error) => return Err(anyhow::anyhow!("{}: {}", path.display(), error)),
        };
        if !meta.is_file() {
            anyhow::bail!("not a file: {}", path.display());
        }
        Ok(meta)
    }

    async fn write_atomic(path: &PathBuf, content: &str, permissions: std::fs::Permissions) -> anyhow::Result<()> {
        let parent = path.parent().filter(|value| !value.as_os_str().is_empty()).ok_or_else(|| anyhow::anyhow!("path has no parent directory: {}", path.display()))?;
        let name = path.file_name().ok_or_else(|| anyhow::anyhow!("path has no file name: {}", path.display()))?;
        let temp = parent.join(format!(".{}.{}.termix-tmp", name.to_string_lossy(), std::process::id()));
        tokio::fs::write(&temp, content).await.with_context(|| format!("write temp file {:?}", temp))?;
        tokio::fs::set_permissions(&temp, permissions).await.with_context(|| format!("set permissions {:?}", temp))?;
        if let Err(error) = tokio::fs::rename(&temp, path).await {
            let _ = tokio::fs::remove_file(&temp).await;
            anyhow::bail!("{} -> {}: {}", temp.display(), path.display(), error);
        }
        Ok(())
    }
}
