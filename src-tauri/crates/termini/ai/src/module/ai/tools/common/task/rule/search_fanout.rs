use std::collections::{HashMap, HashSet};

pub const FANOUT_THRESHOLD: usize = 3;

#[derive(Default)]
pub struct FanoutTracker {
    by_query: HashMap<String, HashSet<String>>,
}

pub fn is_single_file(root: &str) -> bool {
    let lower = root.trim().to_lowercase();
    let Some(name) = lower.rsplit(['/', '\\']).next() else {
        return false;
    };
    name.contains('.') && !name.starts_with('.') && !lower.ends_with('/')
}

fn query_key(args: &serde_json::Value) -> Option<String> {
    let glob = args.get("glob").and_then(|value| value.as_str()).unwrap_or("");
    if let Some(list) = args.get("queries").and_then(|value| value.as_array()) {
        let joined: Vec<&str> = list.iter().filter_map(|item| item.as_str()).collect();
        if joined.is_empty() {
            return None;
        }
        return Some(format!("{}\u{1}{}", joined.join("\u{2}"), glob));
    }
    let single = args.get("query").and_then(|value| value.as_str())?;
    Some(format!("{}\u{1}{}", single, glob))
}

#[derive(Debug, PartialEq, Eq)]
pub enum FanoutVerdict {
    Allow,
    Fanout { files: Vec<String> },
}

impl FanoutTracker {
    pub fn observe(&mut self, args: &serde_json::Value) -> FanoutVerdict {
        let Some(root) = args.get("root").and_then(|value| value.as_str()) else {
            return FanoutVerdict::Allow;
        };
        if !is_single_file(root) {
            return FanoutVerdict::Allow;
        }
        let Some(key) = query_key(args) else {
            return FanoutVerdict::Allow;
        };
        let roots = self.by_query.entry(key).or_default();
        roots.insert(root.replace('\\', "/"));
        if roots.len() < FANOUT_THRESHOLD {
            return FanoutVerdict::Allow;
        }
        let mut files: Vec<String> = roots.iter().cloned().collect();
        files.sort();
        FanoutVerdict::Fanout { files }
    }
}
