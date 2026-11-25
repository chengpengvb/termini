use std::collections::HashSet;
use code_index_bench::module::ai::tools::common::task::rule::read_dedup::{ranges_of, split_seen, ReadRange};

fn record(seen: &mut HashSet<ReadRange>, args: serde_json::Value) -> (usize, usize) {
    let (dup, fresh) = split_seen(seen, &HashSet::new(), ranges_of(&args));
    for range in &fresh {
        seen.insert(range.clone());
    }
    (dup.len(), fresh.len())
}

#[test]
fn replays_the_observed_loop_and_catches_second_pass() {
    let args = serde_json::json!({
        "root": "C:/project/cpp/myBiz/util/move",
        "files": [
            {"path": "SpeedManager.cpp", "offset": 27, "limit": 140},
            {"path": "FlowTemplates.h", "offset": 1, "limit": 95}
        ]
    });
    let mut seen = HashSet::new();
    assert_eq!(record(&mut seen, args.clone()), (0, 2), "第一次全是新的");
    assert_eq!(record(&mut seen, args.clone()), (2, 0), "第二次一模一样, 必须全判为重复");
}

#[test]
fn single_path_form_of_the_loop_is_caught_too() {
    let args = serde_json::json!({"path": "C:/project/cpp/myBiz/util/move/MouseMotion.cpp", "offset": 114, "limit": 85});
    let mut seen = HashSet::new();
    assert_eq!(record(&mut seen, args.clone()), (0, 1));
    assert_eq!(record(&mut seen, args.clone()), (1, 0));
}

#[test]
fn injected_internal_call_id_does_not_break_matching() {
    let first = serde_json::json!({"root": "C:/repo", "files": [{"path": "a.h", "limit": 25}]});
    let second = serde_json::json!({"root": "C:/repo", "files": [{"path": "a.h", "limit": 25}], "__internal_call_id": "xyz"});
    let mut seen = HashSet::new();
    assert_eq!(record(&mut seen, first), (0, 1));
    assert_eq!(record(&mut seen, second), (1, 0));
}
