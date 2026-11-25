use termini_ai::module::ai::tools::common::task::rule::search_nomatch::{NoMatchTracker, NoMatchVerdict, NOMATCH_THRESHOLD};

fn grouped(root: &str, queries: &[&str]) -> serde_json::Value {
    serde_json::json!({"root": root, "queries": queries})
}

fn grouped_result(empty: &[&str], hit: &[&str]) -> String {
    let mut out = String::from("[code_search] root (n 个关键字)\n");
    for query in empty {
        out.push_str(&format!("\n## {}\n(no matches)\n", query));
    }
    for query in hit {
        out.push_str(&format!("\n## {} (2 matches in 1 files)\na.cpp:3:{}\n", query, query));
    }
    out
}

#[test]
fn a_keyword_missing_in_enough_roots_is_dropped_next_time() {
    let mut tracker = NoMatchTracker::default();
    tracker.record(&grouped("C:/p/a.cpp", &["ZOOM_RATIO", "mapCos", "findPic"]), &grouped_result(&["ZOOM_RATIO", "mapCos"], &["findPic"]));
    assert!(!tracker.is_known_missing("ZOOM_RATIO"), "只在一个 root 上空过还不能下结论");
    tracker.record(&grouped("C:/p/b.h", &["ZOOM_RATIO", "mapCos"]), &grouped_result(&["ZOOM_RATIO", "mapCos"], &[]));
    assert!(tracker.is_known_missing("ZOOM_RATIO"));
    assert!(!tracker.is_known_missing("findPic"), "命中过的不算");

    let verdict = tracker.filter(&grouped("C:/p/c.cpp", &["ZOOM_RATIO", "mapCos", "setDict"]));
    let NoMatchVerdict::Rewrite { args, dropped } = verdict else {
        panic!("应剔掉已知空关键字后继续: {verdict:?}");
    };
    assert_eq!(dropped, vec!["ZOOM_RATIO", "mapCos"]);
    assert_eq!(args["queries"], serde_json::json!(["setDict"]));
}

#[test]
fn all_keywords_known_missing_skips_the_call() {
    let mut tracker = NoMatchTracker::default();
    for root in ["C:/p/a.cpp", "C:/p/b.cpp"] {
        tracker.record(&serde_json::json!({"root": root, "query": "onnx"}), "[code_search] root\n(no matches)");
    }
    assert!(matches!(tracker.filter(&serde_json::json!({"root": "C:/p/c.cpp", "query": "onnx"})), NoMatchVerdict::Skip { .. }));
}

#[test]
fn same_root_repeated_does_not_count_twice() {
    let mut tracker = NoMatchTracker::default();
    for _ in 0..NOMATCH_THRESHOLD + 1 {
        tracker.record(&serde_json::json!({"root": "C:/p/a.cpp", "query": "onnx"}), "[code_search] root\n(no matches)");
    }
    assert!(!tracker.is_known_missing("onnx"), "同一个 root 反复搜不算多个证据");
    assert_eq!(tracker.filter(&serde_json::json!({"root": "C:/p/z.cpp", "query": "onnx"})), NoMatchVerdict::Allow);
}
