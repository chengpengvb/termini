use std::fs;
use code_index_bench::module::ai::code_index::index_guard::{normalize_index_dirs, validate_index_dir, MAX_INDEX_DIRS};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("termini-guard-test").join(name);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn rejects_drive_or_filesystem_root() {
    #[cfg(windows)]
    {
        assert!(validate_index_dir("C:/").is_err(), "C:/ 必须被拒绝");
        assert!(validate_index_dir("C:\\").is_err(), "C:\\ 必须被拒绝");
        assert!(validate_index_dir("D:\\").is_err(), "D:\\ 必须被拒绝");
    }
    #[cfg(not(windows))]
    assert!(validate_index_dir("/").is_err(), "/ 必须被拒绝");
}

#[test]
fn rejects_system_dirs() {
    #[cfg(windows)]
    {
        assert!(validate_index_dir("C:/Windows").is_err());
        assert!(validate_index_dir("C:/Windows/System32").is_err());
        assert!(validate_index_dir("C:/Program Files").is_err());
    }
    #[cfg(target_os = "macos")]
    {
        assert!(validate_index_dir("/System").is_err());
        assert!(validate_index_dir("/usr/local").is_err());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        assert!(validate_index_dir("/usr").is_err());
        assert!(validate_index_dir("/etc").is_err());
    }
}

#[test]
fn rejects_home_root_but_allows_children() {
    let home = if cfg!(windows) { std::env::var("USERPROFILE") } else { std::env::var("HOME") };
    let Ok(home) = home else {
        return;
    };
    assert!(validate_index_dir(&home).is_err(), "主目录本身必须被拒绝");
}

#[test]
fn rejects_relative_and_missing() {
    assert!(validate_index_dir("relative/path").is_err());
    assert!(validate_index_dir("").is_err());
    let missing = std::env::temp_dir().join("termini-guard-does-not-exist-xyz");
    assert!(validate_index_dir(&missing.to_string_lossy()).is_err());
}

#[test]
fn accepts_real_project_dir() {
    let dir = temp_dir("proj-a");
    let ok = validate_index_dir(&dir.to_string_lossy());
    assert!(ok.is_ok(), "普通项目目录应当通过: {:?}", ok.err());
}

#[test]
fn rejects_child_when_parent_configured() {
    let outer = temp_dir("nest");
    let inner = temp_dir("nest/inner");
    let raw = vec![outer.to_string_lossy().to_string(), inner.to_string_lossy().to_string()];
    let (kept, rejected) = normalize_index_dirs(&raw);
    assert_eq!(kept.len(), 1, "包含关系只应保留先配的那个, kept={:?}", kept);
    assert_eq!(rejected.len(), 1);
    assert!(rejected[0].contains("包含关系"), "原因应说明包含关系: {}", rejected[0]);
}

#[test]
fn rejects_parent_when_child_configured() {
    let inner = temp_dir("nest2/inner");
    let outer = temp_dir("nest2");
    // 先内层后外层: 外层也要被拒, 不能悄悄顶掉用户已配的内层
    let raw = vec![inner.to_string_lossy().to_string(), outer.to_string_lossy().to_string()];
    let (kept, rejected) = normalize_index_dirs(&raw);
    assert_eq!(kept.len(), 1, "kept={:?}", kept);
    assert!(kept[0].to_lowercase().replace('\\', "/").ends_with("nest2/inner"), "应保留先配的内层, kept={:?}", kept);
    assert_eq!(rejected.len(), 1);
    assert!(rejected[0].contains("包含关系"), "原因应说明包含关系: {}", rejected[0]);
}

#[test]
fn rejects_artifact_dirs() {
    for name in ["target", "node_modules", "build", "dist", "cmake-build-debug", "CMakeFiles", "__pycache__"] {
        let dir = temp_dir(&format!("artifact/{}", name));
        let result = validate_index_dir(&dir.to_string_lossy());
        assert!(result.is_err(), "{} 应被拒绝", name);
        let message = format!("{}", result.err().unwrap());
        assert!(message.contains("构建产物"), "{} 的原因应说明是构建产物: {}", name, message);
    }
}

#[test]
fn dedups_same_dir() {
    let dir = temp_dir("dup");
    let text = dir.to_string_lossy().to_string();
    let raw = vec![text.clone(), text.replace('/', "\\"), text.to_uppercase()];
    let (kept, _) = normalize_index_dirs(&raw);
    assert_eq!(kept.len(), 1, "同一目录的不同写法应去重, kept={:?}", kept);
}

#[test]
fn caps_at_max() {
    let mut raw = Vec::new();
    for index in 0..(MAX_INDEX_DIRS + 3) {
        raw.push(temp_dir(&format!("cap-{}", index)).to_string_lossy().to_string());
    }
    let (kept, rejected) = normalize_index_dirs(&raw);
    assert_eq!(kept.len(), MAX_INDEX_DIRS, "应截到上限, kept={}", kept.len());
    assert_eq!(rejected.len(), 3, "超出的应给出原因");
    assert!(rejected[0].contains("上限"), "原因应说明超上限: {}", rejected[0]);
}
