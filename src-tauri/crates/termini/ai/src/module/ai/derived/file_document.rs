use std::path::Path;
use std::future::Future;
use summer::tracing::info;
use anyhow::{anyhow, Result};
use crate::module::ai::derived::derived_store::{Derived, DerivedKind, DerivedStore};

impl DerivedStore {
    pub async fn ensure_file_document<F>(&self, kind: DerivedKind, path: &Path, refresh: bool, build: F) -> Result<(Derived, String)>
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        self.ensure_file_document_with(kind, path, refresh, || async move { tokio::task::spawn_blocking(build).await.map_err(|error| error.to_string())? }).await
    }

    pub async fn ensure_file_document_with<F, Fut>(&self, kind: DerivedKind, path: &Path, refresh: bool, build: F) -> Result<(Derived, String)>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<String, String>>,
    {
        let bytes = tokio::fs::read(path).await.map_err(|error| anyhow!("read {}: {}", path.display(), error))?;
        let source_hash = Self::content_hash(&bytes);
        let size = bytes.len() as u64;
        drop(bytes);
        let key = Self::file_key(path);
        let variant = Self::variant_of(None);
        if let Some(cached) = self.lookup(&key, &variant, Some(&source_hash), refresh) {
            let content = Self::read_content(&cached)?;
            info!("[ai] 命中文档缓存 kind={:?} path={} derived={} lines={}", kind, path.display(), cached.path.display(), cached.meta.lines);
            return Ok((cached, content));
        }
        let started = std::time::Instant::now();
        let full = build().await.map_err(|error| anyhow!("{}", error))?;
        let derived = self.save(&key, &variant, kind, &path.to_string_lossy(), Some(source_hash), Some(size), None, None, None, &full)?;
        info!("[ai] 文档全文已落盘 kind={:?} path={} derived={} lines={} 耗时={}ms", kind, path.display(), derived.path.display(), derived.meta.lines, started.elapsed().as_millis());
        Ok((derived, full))
    }
}
