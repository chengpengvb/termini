use std::collections::HashMap;

pub fn merge_env(base: &[(String, String)], extra: &[(String, String)]) -> Vec<(String, String)> {
    let mut merged: HashMap<String, String> = HashMap::new();
    for (key, value) in base {
        merged.insert(key.clone(), value.clone());
    }
    for (key, value) in extra {
        merged.insert(key.clone(), value.clone());
    }
    merged.into_iter().collect()
}
