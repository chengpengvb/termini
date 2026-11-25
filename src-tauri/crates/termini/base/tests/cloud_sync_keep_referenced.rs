use serde_json::{json, Value};
use std::collections::HashSet;
use termini_base::service::service_base_sync::keep_referenced;

fn ids(items: &[Value]) -> Vec<String> {
    items.iter().map(|item| item.get("id").and_then(Value::as_str).expect("id").to_string()).collect()
}

#[test]
fn keep_referenced_none_referenced_keeps_first() {
    let merged = vec![json!({"id": "b", "name": "dup"}), json!({"id": "a", "name": "dup"})];
    let referenced = HashSet::new();
    let out = keep_referenced(merged, "name", &referenced).expect("keep");
    assert_eq!(ids(&out), vec!["b".to_string()]);
}

#[test]
fn keep_referenced_one_referenced_keeps_that_id() {
    let merged = vec![json!({"id": "a", "name": "dup"}), json!({"id": "b", "name": "dup"})];
    let referenced = HashSet::from(["b".to_string()]);
    let out = keep_referenced(merged, "name", &referenced).expect("keep");
    assert_eq!(ids(&out), vec!["b".to_string()]);
}

#[test]
fn keep_referenced_both_referenced_keeps_both() {
    let merged = vec![json!({"id": "a", "name": "dup"}), json!({"id": "b", "name": "dup"})];
    let referenced = HashSet::from(["a".to_string(), "b".to_string()]);
    let out = keep_referenced(merged, "name", &referenced).expect("keep");
    assert_eq!(ids(&out), vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn keep_referenced_job_device_id_field() {
    let merged = vec![
        json!({"id": "jd-unref", "device_id": "dev-1"}),
        json!({"id": "jd-ref", "device_id": "dev-1"}),
    ];
    let referenced = HashSet::from(["jd-ref".to_string()]);
    let out = keep_referenced(merged, "device_id", &referenced).expect("keep");
    assert_eq!(ids(&out), vec!["jd-ref".to_string()]);
}

#[test]
fn keep_referenced_distinct_keys_all_kept() {
    let merged = vec![json!({"id": "g1", "name": "one"}), json!({"id": "g2", "name": "two"})];
    let referenced = HashSet::new();
    let out = keep_referenced(merged, "name", &referenced).expect("keep");
    assert_eq!(ids(&out), vec!["g1".to_string(), "g2".to_string()]);
}

#[test]
fn duplicate_error_code_is_40093() {
    use termini_core::model::resp::resp_model::{BizError, ResultCode};
    let err = anyhow::Error::from(BizError::duplicate("existing-id".to_string()));
    let biz = err.downcast_ref::<BizError>().expect("biz");
    assert_eq!(biz.code, ResultCode::Duplicate.as_i32());
    assert_eq!(biz.message, "existing-id");
}
