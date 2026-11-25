use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use termini_core::util::path_util;
use objc2_foundation::{NSSearchPathDirectory, NSSearchPathDomainMask, NSSearchPathForDirectoriesInDomains};

pub fn documents_dir() -> Result<PathBuf> {
    let paths = NSSearchPathForDirectoriesInDomains(NSSearchPathDirectory::DocumentDirectory, NSSearchPathDomainMask::UserDomainMask, true);
    let path = paths.firstObject().ok_or_else(|| anyhow!("Documents directory unavailable"))?;
    Ok(PathBuf::from(path.to_string()))
}

pub fn runtime_dir() -> Result<PathBuf> {
    Ok(documents_dir()?.join("Termini").join(".runtime"))
}

pub fn ensure_within_documents(path: &str) -> Result<()> {
    let root = resolve_compare_path(&documents_dir()?)?;
    let candidate = resolve_compare_path(Path::new(path))?;
    if candidate == root || candidate.starts_with(&root) {
        return Ok(());
    }
    Err(anyhow!("path is outside the local documents directory: {} (resolved={}, documents={})", path, candidate.display(), root.display()))
}

pub fn is_within_documents(path: &str) -> bool {
    ensure_within_documents(path).is_ok()
}

fn resolve_compare_path(path: &Path) -> Result<PathBuf> {
    path_util::resolve_path_for_scope_check(path).map_err(|error| anyhow!("path canonicalize failed: {}: {}", path.display(), error))
}
