use termini_ai::module::ai::tools::common::task::rule::search_fanout::{is_single_file, FanoutTracker, FanoutVerdict, FANOUT_THRESHOLD};

fn search(root: &str, query: &str) -> serde_json::Value {
    serde_json::json!({"root": root, "query": query})
}

#[test]
fn directory_roots_are_never_fanout() {
    let mut tracker = FanoutTracker::default();
    for dir in ["C:/repo/service", "C:/repo/util", "C:/repo/pont", "C:/repo/ui"] {
        assert_eq!(tracker.observe(&search(dir, "class ")), FanoutVerdict::Allow);
    }
}

#[test]
fn same_query_over_many_single_files_trips_at_threshold() {
    let mut tracker = FanoutTracker::default();
    assert_eq!(tracker.observe(&search("C:/repo/A.h", "virtual ")), FanoutVerdict::Allow);
    assert_eq!(tracker.observe(&search("C:/repo/B.h", "virtual ")), FanoutVerdict::Allow);
    let third = tracker.observe(&search("C:/repo/C.h", "virtual "));
    let FanoutVerdict::Fanout { files } = third else {
        panic!("第 {} 个单文件就该拦, 实际 {:?}", FANOUT_THRESHOLD, third);
    };
    assert_eq!(files, vec!["C:/repo/A.h", "C:/repo/B.h", "C:/repo/C.h"]);
}

#[test]
fn different_queries_do_not_pool_together() {
    let mut tracker = FanoutTracker::default();
    tracker.observe(&search("C:/repo/A.h", "virtual "));
    tracker.observe(&search("C:/repo/B.h", "class "));
    // 两个不同正则各只用过一次, 都不该拦
    assert_eq!(tracker.observe(&search("C:/repo/C.h", "struct ")), FanoutVerdict::Allow);
}

#[test]
fn repeating_the_same_file_does_not_inflate_count() {
    let mut tracker = FanoutTracker::default();
    for _ in 0..5 {
        assert_eq!(tracker.observe(&search("C:/repo/A.h", "virtual ")), FanoutVerdict::Allow);
    }
}

#[test]
fn batch_queries_and_glob_are_part_of_the_key() {
    let mut tracker = FanoutTracker::default();
    let batch = |root: &str| serde_json::json!({"root": root, "queries": ["class ", "virtual "], "glob": "*.h"});
    tracker.observe(&batch("C:/repo/A.h"));
    tracker.observe(&batch("C:/repo/B.h"));
    assert!(matches!(tracker.observe(&batch("C:/repo/C.h")), FanoutVerdict::Fanout { .. }));
    let other = serde_json::json!({"root": "C:/repo/D.h", "queries": ["class ", "virtual "], "glob": "*.cpp"});
    assert_eq!(tracker.observe(&other), FanoutVerdict::Allow);
}

#[test]
fn dotted_directory_names_are_not_files() {
    assert!(!is_single_file("C:/repo/.git"));
    assert!(!is_single_file("C:/repo/src/"));
    assert!(is_single_file("C:/repo/src/main.cpp"));
    assert!(is_single_file(r"C:\repo\include\entity.h"));
}
