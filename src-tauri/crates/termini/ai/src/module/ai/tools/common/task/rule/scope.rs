pub fn normalize(path: &str) -> String {
    path.trim().replace('\\', "/").trim_end_matches('/').to_lowercase()
}

pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || (path.len() > 2 && path.as_bytes()[1] == b':')
}

pub fn join(root: Option<&str>, path: &str) -> String {
    if is_absolute(path) {
        return path.to_string();
    }
    match root {
        Some(root) => format!("{}/{}", root.trim_end_matches('/'), path),
        None => path.to_string(),
    }
}

pub fn directories_of(scope: &[String], is_file: impl Fn(&str) -> bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in scope {
        let trimmed = entry.trim().trim_end_matches(['/', '\\']);
        if trimmed.is_empty() {
            continue;
        }
        let dir = if is_file(trimmed) {
            match trimmed.rfind(['/', '\\']) {
                Some(cut) if cut > 0 => &trimmed[..cut],
                _ => trimmed,
            }
        } else {
            trimmed
        };
        if !out.iter().any(|seen| normalize(seen) == normalize(dir)) {
            out.push(dir.to_string());
        }
    }
    out
}

pub fn touched_paths(args: &serde_json::Value) -> Vec<String> {
    let root = args.get("root").and_then(|value| value.as_str());
    let mut out = Vec::new();
    if let Some(path) = args.get("path").and_then(|value| value.as_str()) {
        out.push(join(root, path));
    }
    if let Some(cwd) = args.get("cwd").and_then(|value| value.as_str()) {
        out.push(cwd.to_string());
    }
    if let Some(command) = args.get("command").and_then(|value| value.as_str()) {
        out.extend(super::shell_readonly::absolute_paths(command));
    }
    for key in ["files", "edits"] {
        let Some(items) = args.get(key).and_then(|value| value.as_array()) else {
            continue;
        };
        for item in items {
            if let Some(path) = item.get("path").and_then(|value| value.as_str()) {
                out.push(join(root, path));
            }
        }
    }
    if out.is_empty() {
        if let Some(root) = root {
            out.push(root.to_string());
        }
    }
    out
}

pub fn outside(scope: &[String], path: &str) -> bool {
    let target = normalize(path);
    !scope.iter().any(|allowed| {
        let allowed = normalize(allowed);
        target == allowed || target.starts_with(&format!("{}/", allowed))
    })
}

pub fn offending_paths(scope: &[String], args: &serde_json::Value) -> Vec<String> {
    if scope.is_empty() {
        return Vec::new();
    }
    touched_paths(args).into_iter().filter(|path| is_absolute(path) && outside(scope, path)).collect()
}

pub fn common_ancestor(scope: &[String]) -> Option<String> {
    let first = scope.first()?;
    let parts: Vec<Vec<&str>> = scope.iter().map(|dir| dir.trim().trim_end_matches(['/', '\\']).split(['/', '\\']).collect()).collect();
    let mut depth = 0usize;
    'outer: while let Some(head) = parts[0].get(depth) {
        for other in &parts[1..] {
            if other.get(depth).map(|item| item.to_lowercase()) != Some(head.to_lowercase()) {
                break 'outer;
            }
        }
        depth += 1;
    }
    if depth == 0 {
        return None;
    }
    let separator = if first.contains('\\') && !first.contains('/') { "\\" } else { "/" };
    Some(parts[0][..depth].join(separator))
}

pub const NARROWABLE_TOOLS: &[&str] = &["code_search", "code_semantic", "code_symbol", "code_map"];

pub fn narrow_to_scope(tool: &str, scope: &[String], args: &serde_json::Value) -> Option<serde_json::Value> {
    if !NARROWABLE_TOOLS.contains(&tool) {
        return None;
    }
    if args.get("path").is_some() || args.get("files").is_some() {
        return None;
    }
    let root = args.get("root").and_then(|value| value.as_str())?;
    let root_norm = normalize(root);
    let under: Vec<&String> = scope.iter().filter(|dir| normalize(dir).starts_with(&format!("{}/", root_norm))).collect();
    if under.is_empty() {
        return None;
    }
    let mut next = args.clone();
    if under.len() == 1 {
        next["root"] = serde_json::json!(under[0]);
        return Some(next);
    }
    if tool != "code_search" {
        return None;
    }
    let rels: Vec<String> = under.iter().map(|dir| dir.trim().replace('\\', "/")[root.trim().trim_end_matches(['/', '\\']).len() + 1..].to_string()).collect();
    next["glob"] = serde_json::json!(format!("{{{}}}/**", rels.join(",")));
    Some(next)
}
