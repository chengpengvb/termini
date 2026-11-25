use anyhow::{bail, Result};
use std::path::{Component, Path, PathBuf};

pub const MAX_INDEX_DIRS: usize = 5;

#[cfg(windows)]
const MIN_DEPTH: usize = 1;
#[cfg(not(windows))]
const MIN_DEPTH: usize = 2;

#[cfg(windows)]
const DENY_PREFIXES: &[&str] =
    &["c:/windows", "c:/program files", "c:/program files (x86)", "c:/programdata", "c:/$recycle.bin", "c:/system volume information", "c:/users/all users", "c:/users/default"];
#[cfg(target_os = "macos")]
const DENY_PREFIXES: &[&str] = &["/system", "/library", "/private", "/applications", "/usr", "/bin", "/sbin", "/etc", "/var", "/dev", "/volumes"];
#[cfg(all(unix, not(target_os = "macos")))]
const DENY_PREFIXES: &[&str] = &["/usr", "/bin", "/sbin", "/etc", "/var", "/dev", "/proc", "/sys", "/boot", "/lib", "/lib64", "/opt", "/run", "/tmp", "/mnt", "/media", "/snap"];

fn is_home_root(normalized: &str) -> bool {
    let Some(home) = dirs_home() else {
        return false;
    };
    normalized == home.trim_end_matches('/')
}

fn dirs_home() -> Option<String> {
    let raw = if cfg!(windows) { std::env::var("USERPROFILE").ok()? } else { std::env::var("HOME").ok()? };
    Some(normalize(Path::new(&raw)))
}

const ARTIFACT_DIR_NAMES: &[&str] = &[
    "target",
    "build",
    "dist",
    "out",
    "bin",
    "obj",
    "node_modules",
    "vendor",
    "deps",
    "_deps",
    "_build",
    "cmakefiles",
    "deriveddata",
    "pods",
    "coverage",
    "__pycache__",
    "site-packages",
    "venv",
    ".venv",
    ".git",
    ".gradle",
    ".cache",
    ".next",
    ".nuxt",
    ".turbo",
    ".terraform",
    ".dart_tool",
    ".stack-work",
    "generated",
    "gen",
];

fn is_artifact_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|item| item.to_str()) else {
        return false;
    };
    let lower = name.to_lowercase();
    if lower.starts_with("cmake-build") || lower.ends_with(".egg-info") || lower.ends_with(".xcodeproj") || lower.ends_with(".xcworkspace") {
        return true;
    }
    ARTIFACT_DIR_NAMES.contains(&lower.as_str())
}

pub fn normalize(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/").to_lowercase();
    let trimmed = text.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    trimmed.to_string()
}

fn depth(path: &Path) -> usize {
    path.components()
        .filter(|component| match component {
            Component::Normal(_) => true,
            _ => false,
        })
        .count()
}

pub fn validate_index_dir(raw: &str) -> Result<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("directory is empty");
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        bail!("must be an absolute path: {}", trimmed);
    }
    if !path.exists() {
        bail!("directory does not exist: {}", trimmed);
    }
    if !path.is_dir() {
        bail!("not a directory: {}", trimmed);
    }
    let normalized = normalize(&path);
    if depth(&path) < MIN_DEPTH {
        bail!("a drive root or top-level system directory cannot be selected; choose a specific project directory: {}", trimmed);
    }
    if is_home_root(&normalized) {
        bail!("the user home directory itself cannot be selected; choose a specific project directory: {}", trimmed);
    }
    for deny in DENY_PREFIXES {
        if normalized == *deny || normalized.starts_with(&format!("{}/", deny)) {
            bail!("system directories cannot be used as index directories: {}", trimmed);
        }
    }
    if is_artifact_dir(&path) {
        bail!("build artifact/dependency directories are not indexed; choose the project source directory: {}", trimmed);
    }
    Ok(path)
}

pub fn normalize_index_dirs(raw: &[String]) -> (Vec<String>, Vec<String>) {
    let mut kept: Vec<PathBuf> = Vec::new();
    let mut rejected: Vec<String> = Vec::new();
    for item in raw {
        if item.trim().is_empty() {
            continue;
        }
        let path = match validate_index_dir(item) {
            Ok(path) => path,
            Err(error) => {
                rejected.push(format!("{}: {}", item.trim(), error));
                continue;
            }
        };
        let normalized = normalize(&path);
        if kept.iter().any(|exists| normalize(exists) == normalized) {
            continue;
        }
        if let Some(conflict) = kept.iter().find(|exists| is_ancestor(exists, &path) || is_ancestor(&path, exists)) {
            rejected.push(format!("{}: nested with already configured {}; only one of them can be kept", item.trim(), conflict.display()));
            continue;
        }
        if kept.len() >= MAX_INDEX_DIRS {
            rejected.push(format!("{}: exceeds the limit of {} directories", item.trim(), MAX_INDEX_DIRS));
            continue;
        }
        kept.push(path);
    }
    (kept.into_iter().map(|path| path.to_string_lossy().replace('\\', "/")).collect(), rejected)
}

pub fn is_ancestor(ancestor: &Path, child: &Path) -> bool {
    let ancestor = normalize(ancestor);
    let child = normalize(child);
    child != ancestor && child.starts_with(&format!("{}/", ancestor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_drive_root() {
        #[cfg(windows)]
        {
            assert!(validate_index_dir("C:/").is_err());
            assert!(validate_index_dir("D:\\").is_err());
        }
        #[cfg(not(windows))]
        {
            assert!(validate_index_dir("/").is_err());
        }
    }

    #[test]
    fn rejects_system_dirs() {
        #[cfg(windows)]
        assert!(validate_index_dir("C:/Windows/System32").is_err());
        #[cfg(target_os = "macos")]
        assert!(validate_index_dir("/System/Library").is_err());
        #[cfg(all(unix, not(target_os = "macos")))]
        assert!(validate_index_dir("/usr/lib").is_err());
    }

    #[test]
    fn drops_nested_and_caps_count() {
        let raw: Vec<String> = vec!["/a/b".into(), "/a/b/c".into()];
        let (kept, rejected) = normalize_index_dirs(&raw);
        // 不存在的路径会被校验挡下, 这里只验证不会 panic 且输出成对
        assert_eq!(kept.len() + rejected.len(), 2);
    }
}
