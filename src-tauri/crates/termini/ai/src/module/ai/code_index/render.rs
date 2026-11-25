use std::path::Path;
use std::collections::HashMap;
use crate::module::ai::code_index::extract::Symbol;
use crate::module::ai::code_index::walker::SearchHit;

const MAX_DIR_DEPTH: usize = 6;
const CHARS_PER_TOKEN: usize = 4;
const MAX_SYMBOL_DEFS: usize = 8;
const MAX_SYMBOL_REFS: usize = 12;
const MAX_DIR_TOP_NAMES: usize = 4;
const DEFAULT_MAX_TOKENS: usize = 2048;
const MAP_TRUNCATED: &str = "… truncated by token budget\n";

pub fn render_map(root: &Path, symbols: &[Symbol], focus: Option<&str>, max_tokens: Option<usize>) -> String {
    let budget = max_tokens.unwrap_or(DEFAULT_MAX_TOKENS).saturating_mul(CHARS_PER_TOKEN).max(512);
    let focus = focus.map(str::trim).filter(|item| !item.is_empty());

    let mut ref_counts: HashMap<&str, usize> = HashMap::new();
    for symbol in symbols.iter().filter(|item| !item.is_def) {
        *ref_counts.entry(symbol.name.as_str()).or_default() += 1;
    }

    let mut groups: HashMap<&str, Vec<&Symbol>> = HashMap::new();
    for symbol in symbols.iter().filter(|item| item.is_def) {
        groups.entry(symbol.path.as_str()).or_default().push(symbol);
    }
    if groups.is_empty() {
        return format!("[code_map] {}\n(no definitions)", root.display());
    }
    let mut grouped: Vec<(String, Vec<&Symbol>)> = groups
        .into_iter()
        .map(|(path, mut defs)| {
            defs.sort_by(|left, right| score_def(right, focus, &ref_counts).cmp(&score_def(left, focus, &ref_counts)).then_with(|| left.start_line.cmp(&right.start_line)));
            (path.to_string(), defs)
        })
        .collect();
    grouped.sort_by(|(left_path, left_defs), (right_path, right_defs)| {
        let left_best = left_defs.first().map(|def| score_def(def, focus, &ref_counts)).unwrap_or(0);
        let right_best = right_defs.first().map(|def| score_def(def, focus, &ref_counts)).unwrap_or(0);
        right_best.cmp(&left_best).then_with(|| left_path.cmp(right_path))
    });

    if let Some(text) = render_file_map(root, &grouped, budget) {
        return text;
    }
    render_dir_map(root, &grouped, focus, &ref_counts, budget)
}

fn render_file_map(root: &Path, grouped: &[(String, Vec<&Symbol>)], budget: usize) -> Option<String> {
    let mut out = format!("[code_map] {}\n", root.display());
    for (path, defs) in grouped {
        if out.len() + path.len() + 1 > budget {
            return None;
        }
        out.push_str(path);
        out.push('\n');
        for def in defs {
            let line = format!("  {}:{} {}\n", def.kind, def.start_line, def.name);
            if out.len() + line.len() > budget {
                return None;
            }
            out.push_str(&line);
        }
    }
    Some(out)
}

fn render_dir_map(root: &Path, grouped: &[(String, Vec<&Symbol>)], focus: Option<&str>, ref_counts: &HashMap<&str, usize>, budget: usize) -> String {
    let deepest = grouped.iter().map(|(path, _)| dir_depth(path)).max().unwrap_or(1).clamp(1, MAX_DIR_DEPTH);
    let (mut best, _) = render_dir_map_at(root, grouped, focus, ref_counts, 1, budget);
    for depth in 2..=deepest {
        let (text, truncated) = render_dir_map_at(root, grouped, focus, ref_counts, depth, budget);
        if truncated {
            break;
        }
        best = text;
    }
    best
}

fn render_dir_map_at<'a>(root: &Path, grouped: &[(String, Vec<&'a Symbol>)], focus: Option<&str>, ref_counts: &HashMap<&str, usize>, depth: usize, budget: usize) -> (String, bool) {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut rows: Vec<(String, usize, usize, i64, Vec<&'a Symbol>)> = Vec::new();
    for (path, defs) in grouped {
        let dir = dir_prefix(path, depth);
        let best = defs.first().map(|def| score_def(def, focus, ref_counts)).unwrap_or(0);
        let tops = defs.iter().take(MAX_DIR_TOP_NAMES).copied();
        match index.get(&dir) {
            Some(&at) => {
                let row = &mut rows[at];
                row.1 += 1;
                row.2 += defs.len();
                row.3 = row.3.max(best);
                row.4.extend(tops);
            }
            None => {
                index.insert(dir.clone(), rows.len());
                rows.push((dir, 1, defs.len(), best, tops.collect()));
            }
        }
    }
    rows.sort_by(|left, right| right.3.cmp(&left.3).then_with(|| left.0.cmp(&right.0)));

    let mut out = format!("[code_map] {} ({} dirs, depth={}, 逐目录下钻可看更细)\n", root.display(), rows.len(), depth);
    let mut truncated = false;
    for (dir, files, defs, _, mut tops) in rows {
        tops.sort_by(|left, right| score_def(right, focus, ref_counts).cmp(&score_def(left, focus, ref_counts)));
        tops.dedup_by(|left, right| left.name == right.name);
        let names: Vec<&str> = tops.iter().take(MAX_DIR_TOP_NAMES).map(|def| def.name.as_str()).collect();
        let line = format!("{} files={} defs={} {}\n", dir, files, defs, names.join(", "));
        if out.len() + line.len() > budget {
            truncated = true;
            break;
        }
        out.push_str(&line);
    }
    if truncated {
        out.push_str(MAP_TRUNCATED);
    }
    (out, truncated)
}

fn dir_depth(path: &str) -> usize {
    path.split(['/', '\\']).count().saturating_sub(1)
}

fn dir_prefix(path: &str, depth: usize) -> String {
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    if parts.len() <= 1 {
        return ".".to_string();
    }
    parts[..depth.min(parts.len() - 1)].join("/")
}

pub fn render_search(root: &Path, hits: &[SearchHit]) -> String {
    if hits.is_empty() {
        return format!("[code_search] {}\n(no matches)", root.display());
    }
    let files = hits.iter().map(|hit| hit.path.as_path()).collect::<std::collections::HashSet<_>>().len();
    let mut out = format!("[code_search] {} ({} matches in {} files)\n", root.display(), hits.len(), files);
    for hit in hits {
        out.push_str(&format!("{}:{}:", relative_display(root, &hit.path), hit.line));
        out.push_str(hit.text.trim_end());
        out.push('\n');
    }
    out
}

pub fn render_search_grouped(root: &Path, queries: &[String], grouped: &[Vec<SearchHit>]) -> String {
    let mut head = format!("[code_search] {} ({} 个关键字)\n", root.display(), queries.len());
    for (index, query) in queries.iter().enumerate() {
        let hits = grouped.get(index).map(Vec::as_slice).unwrap_or(&[]);
        if hits.is_empty() {
            head.push_str(&format!("\n## {}\n(no matches)\n", query));
            continue;
        }
        let files = hits.iter().map(|hit| hit.path.as_path()).collect::<std::collections::HashSet<_>>().len();
        head.push_str(&format!("\n## {} ({} matches in {} files)\n", query, hits.len(), files));
        head.push_str(&render_group(root, hits));
    }
    head
}

pub fn render_group(root: &Path, hits: &[SearchHit]) -> String {
    if hits.is_empty() {
        return "(no matches)".to_string();
    }
    let mut out = String::new();
    for hit in hits {
        let first = hit.line.saturating_sub(hit.before.len());
        for (index, line) in hit.before.iter().enumerate() {
            out.push_str(&format!("  {:>5}  {}\n", first + index, line.trim_end()));
        }
        out.push_str(&format!("{}:{}:{}\n", relative_display(root, &hit.path), hit.line, hit.text.trim_end()));
        for (index, line) in hit.after.iter().enumerate() {
            out.push_str(&format!("  {:>5}  {}\n", hit.line + 1 + index, line.trim_end()));
        }
    }
    out
}

fn relative_display(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    if relative.as_os_str().is_empty() {
        return path.to_string_lossy().replace('\\', "/");
    }
    relative.to_string_lossy().replace('\\', "/")
}

pub fn render_symbol(root: &Path, symbols: &[Symbol], name: &str, kind: Option<&str>, offset: usize) -> String {
    let name = name.trim();
    let kind = kind.map(str::trim).filter(|item| !item.is_empty());
    let needle = name.to_lowercase();
    let mut defs: Vec<&Symbol> = symbols
        .iter()
        .filter(|item| item.is_def)
        .filter(|item| kind.map(|want| item.kind == want).unwrap_or(true))
        .filter(|item| item.name == name || item.name.to_lowercase().contains(&needle))
        .collect();
    if defs.is_empty() {
        return format!("[code_symbol] {}\nname={} kind={}\ntotal=0\n(no definitions)", root.display(), name, kind.unwrap_or("*"));
    }
    defs.sort_by(|left, right| {
        let left_rank = usize::from(left.name != name);
        let right_rank = usize::from(right.name != name);
        left_rank.cmp(&right_rank).then_with(|| left.name.len().cmp(&right.name.len())).then_with(|| left.path.cmp(&right.path)).then_with(|| left.start_line.cmp(&right.start_line))
    });

    let mut refs_by_key: HashMap<(&str, &str), Vec<&Symbol>> = HashMap::new();
    for symbol in symbols.iter().filter(|item| !item.is_def) {
        refs_by_key.entry((symbol.name.as_str(), symbol.path.as_str())).or_default().push(symbol);
    }

    let total = defs.len();
    let page: Vec<&Symbol> = defs.iter().copied().skip(offset).take(MAX_SYMBOL_DEFS).collect();
    let mut out = format!("[code_symbol] {}\nname={} total={} offset={}\n", root.display(), name, total, offset);
    for (index, def) in page.iter().enumerate() {
        out.push_str(&format!("{}. {}:{} {} {}\n", offset + index + 1, def.path, def.start_line, def.kind, def.name));
        match refs_by_key.get(&(def.name.as_str(), def.path.as_str())) {
            None => out.push_str("   refs: (none in same file)\n"),
            Some(items) => {
                let lines: Vec<String> = items.iter().take(MAX_SYMBOL_REFS).map(|item| format!("{}:{}", item.start_line, item.kind)).collect();
                out.push_str(&format!("   refs: {}\n", lines.join(", ")));
                if items.len() > MAX_SYMBOL_REFS {
                    out.push_str(&format!("   … {} more refs\n", items.len() - MAX_SYMBOL_REFS));
                }
            }
        }
    }
    let remaining = total.saturating_sub(offset.saturating_add(page.len()));
    if remaining > 0 {
        out.push_str(&format!("… {} more definitions\n", remaining));
    }
    out
}

fn score_def(def: &Symbol, focus: Option<&str>, ref_counts: &HashMap<&str, usize>) -> i64 {
    let refs = ref_counts.get(def.name.as_str()).copied().unwrap_or(0).min(25) as i64;
    let mut score = refs * 10;
    match def.kind.as_str() {
        "function" | "method" | "class" | "struct" | "trait" | "interface" | "impl" => score += 5,
        "module" | "type" | "enum" => score += 3,
        _ => {}
    }
    if let Some(focus) = focus {
        if def.path.contains(focus) || def.name.contains(focus) {
            score += 1000;
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol(path: &str, name: &str, start_line: usize, is_def: bool) -> Symbol {
        Symbol { path: path.to_string(), name: name.to_string(), kind: if is_def { "function" } else { "call" }.to_string(), start_line, is_def }
    }

    #[test]
    fn map_emits_each_path_once() {
        let symbols = vec![
            symbol("src/a.rs", "alpha", 1, true),
            symbol("src/a.rs", "beta", 5, true),
            symbol("src/b.rs", "gamma", 2, true),
            symbol("src/b.rs", "delta", 9, true),
            symbol("src/a.rs", "alpha", 20, false),
            symbol("src/b.rs", "gamma", 20, false),
        ];
        let rendered = render_map(Path::new("repo"), &symbols, None, Some(512));
        assert_eq!(rendered.matches("src/a.rs\n").count(), 1);
        assert_eq!(rendered.matches("src/b.rs\n").count(), 1);
        assert!(rendered.contains("function:1 alpha"));
        assert!(rendered.contains("function:9 delta"));
    }
}
