use std::path::{Component, Path, PathBuf};
use crate::module::ai::tools::ai_tool_error::AiToolError;

const GIT_DIR: &str = ".git";
const DATA_DIR_ENV: &str = "TERMINI_DATA_DIR";

pub struct PathGuard;

impl PathGuard {
    pub fn ensure_writable(path: &Path) -> Result<(), AiToolError> {
        let resolved = Self::resolve(path);
        if resolved.components().any(|component| matches!(component, Component::Normal(name) if name == GIT_DIR)) {
            return Err(Self::refuse(path, "path is inside a .git directory"));
        }
        if let Some(data_dir) = std::env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty()).map(PathBuf::from) {
            if resolved.starts_with(Self::resolve(&data_dir)) {
                return Err(Self::refuse(path, "path is inside the application data directory"));
            }
        }
        #[cfg(target_os = "ios")]
        {
            if let Err(error) = termini_platform::util::ios_local_dir::ensure_within_documents(resolved.to_string_lossy().as_ref()) {
                return Err(Self::refuse(path, &error.to_string()));
            }
        }
        Ok(())
    }

    fn refuse(path: &Path, reason: &str) -> AiToolError {
        AiToolError::Execution(format!("refusing to write inside protected path: {} ({})", path.display(), reason))
    }

    fn resolve(path: &Path) -> PathBuf {
        let normalized = Self::normalize_lexically(path);
        Self::canonicalize_existing_prefix(&normalized).unwrap_or(normalized)
    }

    fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
        let mut existing = path;
        let mut rest = Vec::new();
        loop {
            if let Ok(canonical) = existing.canonicalize() {
                let mut out = canonical;
                for part in rest.iter().rev() {
                    out.push(part);
                }
                return Some(out);
            }
            rest.push(existing.file_name()?);
            existing = existing.parent()?;
        }
    }

    fn normalize_lexically(path: &Path) -> PathBuf {
        let mut out = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                }
                other => out.push(other.as_os_str()),
            }
        }
        out
    }
}
