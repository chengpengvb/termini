use std::io;
use std::path::{Component, Path, PathBuf};

pub fn normalize_to_slash(path: &str) -> String {
    path.replace('\\', "/")
}

pub fn strip_unc_prefix(path: String) -> String {
    #[cfg(windows)]
    {
        if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{}", rest);
        }
        if let Some(rest) = path.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
    }
    path
}

pub fn normalize_frontend_path(path: &str) -> String {
    normalize_to_slash(strip_unc_prefix(path.to_string()).as_str())
}

pub fn normalize_pathbuf_lossy(path: &Path) -> String {
    normalize_frontend_path(path.to_string_lossy().as_ref())
}

pub fn is_cross_device(error: &io::Error) -> bool {
    #[cfg(unix)]
    {
        const EXDEV: i32 = 18;
        return error.raw_os_error() == Some(EXDEV);
    }
    #[cfg(windows)]
    {
        const ERROR_NOT_SAME_DEVICE: i32 = 17;
        return error.raw_os_error() == Some(ERROR_NOT_SAME_DEVICE);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = error;
        false
    }
}

pub fn join_frontend_path(base: &str, name: &str) -> String {
    let trimmed_base = base.trim_end_matches('/');
    let trimmed_name = name.trim_start_matches('/');
    format!("{}/{}", trimmed_base, trimmed_name)
}

pub fn is_frontend_absolute_path(path: &str) -> bool {
    if path.starts_with('/') {
        return true;
    }
    if path.starts_with("//") {
        return true;
    }
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        return true;
    }
    false
}

pub fn is_computer_root(path: &str) -> bool {
    normalize_frontend_path(path) == "/"
}

pub fn is_windows_drive_root(path: &str) -> bool {
    let normalized = normalize_frontend_path(path);
    let trimmed = normalized.trim_end_matches('/');
    let bytes = trimmed.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

pub fn unix_volume_root() -> &'static str {
    "/Volumes"
}

pub fn is_unix_volume_root(path: &str) -> bool {
    normalize_frontend_path(path).trim_end_matches('/') == "/Volumes"
}

pub fn parent_frontend_path(path: &str) -> Option<String> {
    let normalized = normalize_frontend_path(path);
    if is_computer_root(&normalized) {
        return None;
    }
    let trimmed = normalized.trim_end_matches('/');
    if is_windows_drive_root(trimmed) {
        return Some("/".to_string());
    }
    let index = trimmed.rfind('/')?;
    if index == 0 {
        return Some("/".to_string());
    }
    Some(trimmed[..index].to_string())
}

pub fn resolve_link_target_path(link_path: &str, target: &str, parent_path: Option<&str>) -> String {
    if is_frontend_absolute_path(target) {
        return normalize_frontend_path(target);
    }
    if let Some(parent) = parent_path {
        return join_frontend_path(parent, target);
    }
    join_frontend_path(link_path, target)
}

/// Resolve a path into a stable form suitable for directory scope checks.
///
/// `canonicalize` cannot resolve a destination that has not been created yet.
/// In that case, canonicalize the closest existing ancestor and append the
/// unresolved tail. This keeps platform aliases (for example iOS `/var` and
/// `/private/var`) consistent without weakening traversal checks.
pub fn resolve_path_for_scope_check(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir()?.join(path) };

    for ancestor in absolute.ancestors() {
        match ancestor.canonicalize() {
            Ok(mut resolved) => {
                let tail = absolute.strip_prefix(ancestor).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
                for component in tail.components() {
                    match component {
                        Component::Normal(value) => resolved.push(value),
                        Component::CurDir => {}
                        Component::ParentDir => {
                            resolved.pop();
                        }
                        Component::Prefix(_) | Component::RootDir => {
                            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unexpected absolute component in unresolved path tail"));
                        }
                    }
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // A dangling symlink also reports NotFound from canonicalize.
                // Do not fall back to lexical resolution when an entry exists,
                // because a later operation could follow it outside the scope.
                match ancestor.symlink_metadata() {
                    Ok(_) => return Err(error),
                    Err(metadata_error) if metadata_error.kind() == io::ErrorKind::NotFound => {}
                    Err(metadata_error) => return Err(metadata_error),
                }
            }
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(io::ErrorKind::NotFound, format!("no existing ancestor for path: {}", absolute.display())))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use super::resolve_path_for_scope_check;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock before unix epoch").as_nanos();
            let path = std::env::temp_dir().join(format!("termini-path-scope-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn canonicalizes_existing_parent_for_new_destination() {
        let temp = TempDir::new();
        let root = temp.0.join("Documents");
        fs::create_dir_all(&root).expect("create documents directory");

        let resolved = resolve_path_for_scope_check(&root.join("new-folder").join("file.txt")).expect("resolve new destination");

        assert_eq!(resolved, root.canonicalize().expect("canonical documents directory").join("new-folder").join("file.txt"));
    }

    #[test]
    fn normalizes_parent_components_in_unresolved_tail() {
        let temp = TempDir::new();
        let root = temp.0.join("Documents");
        fs::create_dir_all(&root).expect("create documents directory");

        let resolved = resolve_path_for_scope_check(&root.join("missing").join("..").join("file.txt")).expect("resolve path with parent component");

        assert_eq!(resolved, root.canonicalize().expect("canonical documents directory").join("file.txt"));
    }

    #[test]
    fn keeps_parent_escape_outside_the_scoped_root() {
        let temp = TempDir::new();
        let root = temp.0.join("Documents");
        fs::create_dir_all(&root).expect("create documents directory");
        let canonical_root = root.canonicalize().expect("canonical documents directory");

        let resolved = resolve_path_for_scope_check(&root.join("missing").join("..").join("..").join("outside.txt")).expect("resolve escaped destination");

        assert!(!resolved.starts_with(canonical_root));
    }

    #[test]
    fn parent_of_computer_root_is_none() {
        assert_eq!(super::parent_frontend_path("/"), None);
    }

    #[test]
    fn parent_of_unix_child_is_computer_root() {
        assert_eq!(super::parent_frontend_path("/root"), Some("/".to_string()));
        assert_eq!(super::parent_frontend_path("/root/foo"), Some("/root".to_string()));
    }

    #[test]
    fn parent_of_windows_drive_is_computer_root() {
        assert_eq!(super::parent_frontend_path("C:/"), Some("/".to_string()));
        assert_eq!(super::parent_frontend_path("C:"), Some("/".to_string()));
        assert_eq!(super::parent_frontend_path("D:/"), Some("/".to_string()));
        assert_eq!(super::parent_frontend_path("C:/Users"), Some("C:".to_string()));
    }

    #[test]
    fn unix_volume_root_matches_volumes() {
        assert!(super::is_unix_volume_root("/Volumes"));
        assert!(super::is_unix_volume_root("/Volumes/"));
        assert!(!super::is_unix_volume_root("/"));
        assert!(!super::is_unix_volume_root("/Volumes/USB"));
    }
}
