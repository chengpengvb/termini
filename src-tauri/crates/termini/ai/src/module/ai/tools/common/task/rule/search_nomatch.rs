use std::collections::{HashMap, HashSet};

pub const NOMATCH_THRESHOLD: usize = 2;

#[derive(Default)]
pub struct NoMatchTracker {
    misses: HashMap<String, HashSet<String>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NoMatchVerdict {
    Allow,
    Rewrite { args: serde_json::Value, dropped: Vec<String> },
    Skip { dropped: Vec<String> },
}

fn queries_of(args: &serde_json::Value) -> Vec<String> {
    if let Some(list) = args.get("queries").and_then(|value| value.as_array()) {
        return list.iter().filter_map(|item| item.as_str()).map(|item| item.to_string()).collect();
    }
    args.get("query").and_then(|value| value.as_str()).map(|item| vec![item.to_string()]).unwrap_or_default()
}

fn root_key(args: &serde_json::Value) -> String {
    args.get("root").and_then(|value| value.as_str()).unwrap_or("").trim().replace('\\', "/").to_lowercase()
}

impl NoMatchTracker {
    pub fn record(&mut self, args: &serde_json::Value, result: &str) {
        let root = root_key(args);
        let queries = queries_of(args);
        if queries.is_empty() {
            return;
        }
        if queries.len() == 1 {
            if result.contains("(no matches)") {
                self.misses.entry(queries[0].clone()).or_default().insert(root);
            }
            return;
        }
        let lines: Vec<&str> = result.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some(head) = line.strip_prefix("## ") else {
                continue;
            };
            let name = head.rsplit_once(" (").map(|(left, _)| left).unwrap_or(head).trim();
            let empty = lines.get(index + 1).is_some_and(|next| next.trim() == "(no matches)");
            if empty && queries.iter().any(|query| query == name) {
                self.misses.entry(name.to_string()).or_default().insert(root.clone());
            }
        }
    }

    pub fn is_known_missing(&self, query: &str) -> bool {
        self.misses.get(query).is_some_and(|roots| roots.len() >= NOMATCH_THRESHOLD)
    }

    pub fn filter(&self, args: &serde_json::Value) -> NoMatchVerdict {
        let queries = queries_of(args);
        if queries.is_empty() {
            return NoMatchVerdict::Allow;
        }
        let dropped: Vec<String> = queries.iter().filter(|query| self.is_known_missing(query)).cloned().collect();
        if dropped.is_empty() {
            return NoMatchVerdict::Allow;
        }
        let kept: Vec<String> = queries.into_iter().filter(|query| !dropped.contains(query)).collect();
        if kept.is_empty() {
            return NoMatchVerdict::Skip { dropped };
        }
        let mut next = args.clone();
        if let Some(map) = next.as_object_mut() {
            map.remove("query");
            map.insert("queries".to_string(), serde_json::json!(kept));
        }
        NoMatchVerdict::Rewrite { args: next, dropped }
    }
}
