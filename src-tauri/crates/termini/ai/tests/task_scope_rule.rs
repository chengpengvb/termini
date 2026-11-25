use termini_ai::module::ai::tools::common::task::rule::scope::{common_ancestor, directories_of, narrow_to_scope, offending_paths, outside, touched_paths};

fn scope(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn file_entries_in_scope_collapse_to_their_parent_directories() {
    let files = scope(&["C:/proj/util/ImageFind.h", "C:/proj/util/ImageFind.cpp", "C:/proj/dto/randRange.h", "C:/proj/util/"]);
    let dirs = directories_of(&files, |path| path.ends_with(".h") || path.ends_with(".cpp"));
    assert_eq!(dirs, vec!["C:/proj/util", "C:/proj/dto"]);
    assert_eq!(directories_of(&scope(&["C:/proj/util"]), |_| false), vec!["C:/proj/util"]);
}

#[test]
fn common_ancestor_of_scope_dirs() {
    let allow = scope(&["C:/repo/saas/center/src/main/java/com/x/job", "C:/repo/saas/center/src/main/java/com/x/config", "C:/repo/saas/common/src"]);
    assert_eq!(common_ancestor(&allow).as_deref(), Some("C:/repo/saas"));
    assert_eq!(common_ancestor(&scope(&["C:/repo/only"])).as_deref(), Some("C:/repo/only"));
    assert_eq!(common_ancestor(&scope(&["C:/a", "D:/b"])), None);
    assert_eq!(common_ancestor(&[]), None);
}

#[test]
fn search_from_ancestor_is_narrowed_to_the_single_scope_dir() {
    let allow = scope(&["C:/repo/saas/gather/src"]);
    let args = serde_json::json!({"root": "C:/repo/saas", "queries": ["x"]});
    let next = narrow_to_scope("code_search", &allow, &args).expect("应收窄而不是拦截");
    assert_eq!(next["root"], "C:/repo/saas/gather/src");
    assert_eq!(next["queries"], serde_json::json!(["x"]));
}

#[test]
fn search_from_ancestor_of_many_dirs_gets_a_brace_glob() {
    let allow = scope(&["C:/repo/saas/center/src", "C:/repo/saas/gather/src"]);
    let args = serde_json::json!({"root": "C:/repo/saas", "query": "x"});
    let next = narrow_to_scope("code_search", &allow, &args).expect("code_search 支持 glob, 应收窄");
    assert_eq!(next["root"], "C:/repo/saas");
    assert_eq!(next["glob"], "{center/src,gather/src}/**");
    assert!(narrow_to_scope("code_map", &allow, &args).is_none(), "code_map 没有 glob, 多目录时只能拦");
}

#[test]
fn unrelated_root_and_file_tools_are_not_narrowed() {
    let allow = scope(&["C:/repo/saas/gather/src"]);
    assert!(narrow_to_scope("code_search", &allow, &serde_json::json!({"root": "C:/other"})).is_none());
    assert!(narrow_to_scope("file_read", &allow, &serde_json::json!({"root": "C:/repo/saas", "files": [{"path": "a"}]})).is_none());
}

#[test]
fn path_inside_scope_is_allowed() {
    let allow = scope(&["C:/repo/gather"]);
    assert!(!outside(&allow, "C:/repo/gather"));
    assert!(!outside(&allow, "C:/repo/gather/src/main"));
}

#[test]
fn sibling_with_shared_prefix_is_outside() {
    let allow = scope(&["C:/repo/gather"]);
    assert!(outside(&allow, "C:/repo/gather2"), "gather2 不是 gather 的子目录");
    assert!(outside(&allow, "C:/repo/center"));
}

#[test]
fn parent_directory_is_outside() {
    assert!(outside(&scope(&["C:/repo/gather"]), "C:/repo"));
}

#[test]
fn separator_and_case_do_not_matter() {
    assert!(!outside(&scope(&["C:/repo/gather"]), r"c:\REPO\Gather\src"));
}

#[test]
fn relative_entries_resolve_against_root() {
    let args = serde_json::json!({"root": "C:/repo", "files": [{"path": "gather/a.java"}, {"path": "center/b.java"}]});
    assert_eq!(touched_paths(&args), vec!["C:/repo/gather/a.java", "C:/repo/center/b.java"]);
    assert_eq!(offending_paths(&scope(&["C:/repo/gather"]), &args), vec!["C:/repo/center/b.java"]);
}

#[test]
fn absolute_entry_ignores_root() {
    let args = serde_json::json!({"root": "C:/repo", "path": "D:/other/x.java"});
    assert_eq!(touched_paths(&args), vec!["D:/other/x.java"]);
}

#[test]
fn root_only_call_uses_root_as_range() {
    let args = serde_json::json!({"root": "C:/repo/center", "query": "foo"});
    assert_eq!(touched_paths(&args), vec!["C:/repo/center"]);
}

#[test]
fn edits_key_is_covered_too() {
    let args = serde_json::json!({"root": "C:/repo", "edits": [{"path": "center/x.java", "old_string": "a", "new_string": "b"}]});
    assert_eq!(offending_paths(&scope(&["C:/repo/gather"]), &args), vec!["C:/repo/center/x.java"]);
}

#[test]
fn relative_path_without_root_is_not_a_scope_violation() {
    // 实测误报: 模型漏传 root 时发了相对路径, 被当成越界; 那该由工具报"必须绝对路径"
    let args = serde_json::json!({"files": [{"path": "center/src/main/java/Rule.java"}]});
    assert!(offending_paths(&scope(&["C:/repo/gather"]), &args).is_empty());
}

#[test]
fn empty_scope_allows_everything() {
    let args = serde_json::json!({"root": "D:/anywhere"});
    assert!(offending_paths(&[], &args).is_empty());
}
