use std::path::{Path, PathBuf};
use crate::module::ai::code_index::index_guard::{normalize, normalize_index_dirs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeHit {
    pub index_root: PathBuf,
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexScope {
    Covered(Vec<ScopeHit>),
    Uncovered,
}

pub fn resolve_index_scope(request: &Path, configured: &[String]) -> IndexScope {
    let req = normalize(request);
    let mut spans: Vec<ScopeHit> = Vec::new();
    for dir in configured {
        if dir.trim().is_empty() {
            continue;
        }
        let index_root = PathBuf::from(dir.trim());
        let cfg = normalize(&index_root);
        if req == cfg {
            return IndexScope::Covered(vec![ScopeHit { index_root, prefix: None }]);
        }
        if let Some(rest) = strip_ancestor(&cfg, &req) {
            return IndexScope::Covered(vec![ScopeHit { index_root, prefix: Some(rest) }]);
        }
        if strip_ancestor(&req, &cfg).is_some() {
            spans.push(ScopeHit { index_root, prefix: None });
        }
    }
    if spans.is_empty() {
        return IndexScope::Uncovered;
    }
    IndexScope::Covered(spans)
}

pub fn scope_from_dirs(request: &Path, raw_dirs: &[String]) -> IndexScope {
    let (valid, _) = normalize_index_dirs(raw_dirs);
    resolve_index_scope(request, &valid)
}

pub fn scan_bases(request: &Path, scope: &IndexScope) -> Vec<PathBuf> {
    match scope {
        IndexScope::Covered(hits) => hits.iter().map(|hit| hit.index_root.clone()).collect(),
        IndexScope::Uncovered => vec![request.to_path_buf()],
    }
}

fn strip_ancestor(ancestor: &str, child: &str) -> Option<String> {
    let head = format!("{}/", ancestor);
    let rest = child.strip_prefix(&head)?;
    if rest.is_empty() {
        return None;
    }
    Some(format!("{}/", rest.trim_end_matches('/')))
}

pub fn path_in_prefix(path: &str, prefix: Option<&String>) -> bool {
    let Some(prefix) = prefix else {
        return true;
    };
    let normalized = path.replace('\\', "/").to_lowercase();
    normalized.starts_with(prefix.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs() -> Vec<String> {
        vec!["C:/project/java/Biz-Shading/saas-monitor".to_string(), "D:/work/api".to_string()]
    }

    #[test]
    fn exact_match_uses_whole_index() {
        let scope = resolve_index_scope(Path::new("C:/project/java/Biz-Shading/saas-monitor"), &dirs());
        let IndexScope::Covered(hits) = scope else {
            panic!("expected covered");
        };
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].prefix, None);
    }

    #[test]
    fn descendant_reuses_parent_index_with_prefix() {
        let scope = resolve_index_scope(Path::new("C:/project/java/Biz-Shading/saas-monitor/center/src/main/java/com/yzfar/monitor/dao"), &dirs());
        let IndexScope::Covered(hits) = scope else {
            panic!("expected covered");
        };
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].index_root, PathBuf::from("C:/project/java/Biz-Shading/saas-monitor"));
        assert_eq!(hits[0].prefix.as_deref(), Some("center/src/main/java/com/yzfar/monitor/dao/"));
    }

    #[test]
    fn ancestor_falls_back_to_configured_children() {
        let scope = resolve_index_scope(Path::new("C:/project/java/Biz-Shading"), &dirs());
        let IndexScope::Covered(hits) = scope else {
            panic!("expected covered");
        };
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].index_root, PathBuf::from("C:/project/java/Biz-Shading/saas-monitor"));
        assert_eq!(hits[0].prefix, None);
    }

    #[test]
    fn ancestor_can_span_several_configured_dirs() {
        let configured = vec!["C:/repo/a".to_string(), "C:/repo/b".to_string()];
        let scope = resolve_index_scope(Path::new("C:/repo"), &configured);
        let IndexScope::Covered(hits) = scope else {
            panic!("expected covered");
        };
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|hit| hit.prefix.is_none()));
    }

    #[test]
    fn unrelated_root_is_uncovered() {
        assert_eq!(resolve_index_scope(Path::new("C:/other/thing"), &dirs()), IndexScope::Uncovered);
    }

    #[test]
    fn sibling_prefix_is_not_a_descendant() {
        let scope = resolve_index_scope(Path::new("C:/project/java/Biz-Shading/saas-monitor2"), &dirs());
        assert_eq!(scope, IndexScope::Uncovered);
    }

    #[test]
    fn separator_and_case_do_not_matter() {
        let scope = resolve_index_scope(Path::new(r"c:\PROJECT\java\Biz-Shading\SAAS-MONITOR\common"), &dirs());
        let IndexScope::Covered(hits) = scope else {
            panic!("expected covered");
        };
        assert_eq!(hits[0].prefix.as_deref(), Some("common/"));
    }

    #[test]
    fn prefix_filter_matches_index_relative_paths() {
        let prefix = Some("center/src/".to_string());
        assert!(path_in_prefix("center/src/main/java/Foo.java", prefix.as_ref()));
        assert!(path_in_prefix(r"center\src\main\java\Foo.java", prefix.as_ref()));
        assert!(!path_in_prefix("common/src/main/java/Bar.java", prefix.as_ref()));
        assert!(path_in_prefix("anything", None));
    }
}
