use std::path::{Path, PathBuf};
use crate::module::ai::code_index::extract::extract_file;
use crate::module::ai::code_index::walker::{language_for_path, walk_source_files};
use crate::module::ai::code_index::{render_map, render_search, render_symbol, resolve_root, search_files};

fn workspace_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn resolve_root_requires_absolute_dir() {
    assert!(resolve_root("").is_err());
    assert!(resolve_root("relative/path").is_err());
    let root = workspace_src();
    let resolved = resolve_root(root.to_str().expect("utf8 path")).expect("resolve src");
    assert_eq!(resolved, root);
}

#[test]
fn search_finds_tool_code_map() {
    let root = workspace_src();
    let hits = search_files(&root, "ToolCodeMap", Some("**/*.rs"), Some(10)).expect("search");
    assert!(!hits.is_empty(), "应能搜到 ToolCodeMap");
    let rendered = render_search(&root, &hits);
    assert!(rendered.contains("ToolCodeMap"));
}

#[test]
fn extract_map_and_symbol_ai_tool_id() {
    let root = workspace_src();
    let files = walk_source_files(&root).expect("walk");
    assert!(!files.is_empty(), "应能扫到源码文件");

    let sample = files.iter().find(|path| path.ends_with("ai_tool_id.rs")).expect("应存在 ai_tool_id.rs");
    let relative = sample.strip_prefix(&root).unwrap().to_string_lossy().to_string();
    let symbols = extract_file(sample, &relative).expect("extract");
    assert!(symbols.iter().any(|item| item.is_def && item.name == "AiToolId"), "应提取到 AiToolId 定义");

    let map = render_map(&root, &symbols, Some("ai_tool_id"), Some(512));
    assert!(map.contains("[code_map]"), "地图应带标题");
    assert!(map.contains("AiToolId"), "地图应包含 AiToolId");

    let symbol = render_symbol(&root, &symbols, "AiToolId", None, 0);
    assert!(symbol.contains("AiToolId"), "符号查找应命中 AiToolId");
    assert!(!symbol.contains("no definitions"), "不应是空结果");
}

#[test]
fn extract_accepts_java_c_yaml_powershell() {
    let dir = std::env::temp_dir().join(format!("termini-code-index-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let cases = [
        ("Sample.java", "public class Sample { void run() {} }\n"),
        ("sample.c", "int main(void) { return 0; }\n"),
        ("sample.yml", "services:\n  app:\n    image: nginx\n"),
        ("sample.ps1", "function Get-Thing { param($Name) Write-Host $Name }\n"),
        ("sample.tf", "resource \"null_resource\" \"x\" {}\n"),
    ];
    for (name, body) in cases {
        let path = dir.join(name);
        std::fs::write(&path, body).expect("write sample");
        let symbols = extract_file(&path, name).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!symbols.is_empty() || name.ends_with(".yml") || name.ends_with(".tf"), "{name} 应能解析出符号: {:?}", symbols);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn language_for_path_covers_common_and_ops() {
    assert_eq!(language_for_path(Path::new("a.java")), Some("java"));
    assert_eq!(language_for_path(Path::new("a.c")), Some("c"));
    assert_eq!(language_for_path(Path::new("a.h")), Some("c"));
    assert_eq!(language_for_path(Path::new("a.cpp")), Some("cpp"));
    assert_eq!(language_for_path(Path::new("a.yml")), Some("yaml"));
    assert_eq!(language_for_path(Path::new("a.yaml")), Some("yaml"));
    assert_eq!(language_for_path(Path::new("a.toml")), Some("toml"));
    assert_eq!(language_for_path(Path::new("a.ps1")), Some("powershell"));
    assert_eq!(language_for_path(Path::new("a.tf")), Some("hcl"));
    assert_eq!(language_for_path(Path::new("a.rb")), Some("ruby"));
    assert_eq!(language_for_path(Path::new("a.lua")), Some("lua"));
    assert_eq!(language_for_path(Path::new("a.php")), Some("php"));
    assert_eq!(language_for_path(Path::new("a.json")), Some("json"));
    assert_eq!(language_for_path(Path::new("Makefile")), Some("make"));
    assert_eq!(language_for_path(Path::new("deploy.sh")), Some("bash"));
    assert_eq!(language_for_path(Path::new("run.bat")), Some("bash"));
}
