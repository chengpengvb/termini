use std::path::Path;

const MAX_ISSUES: usize = 8;
const TABLE_MAX_COLUMNS: usize = 8;
const PAGE_BREAK_MARK: &str = "\\newpage";
const RAW_HTML_TAGS: [&str; 8] = ["<table", "<div", "<span", "<p ", "<p>", "<img", "<ul", "<ol"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DocTarget {
    Docx,
    Pdf,
}

pub struct Normalized {
    pub markdown: String,
    pub issues: Vec<String>, // 模型能改的结构问题, 空表示 Markdown 合规
}

pub fn normalize(markdown: &str, base_dir: Option<&Path>, target: DocTarget) -> Normalized {
    let mut issues = Vec::new();
    let mut out: Vec<String> = Vec::new();
    let mut in_fence = false;
    let mut blank_run = 0usize;
    let mut last_heading = 0usize;
    let mut table: Option<TableState> = None;

    for raw in markdown.replace("\r\n", "\n").lines() {
        let line = raw.to_string();
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            out.push(line);
            blank_run = 0;
            continue;
        }
        if in_fence {
            out.push(line);
            continue;
        }
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                out.push(String::new());
            }
            finish_table(&mut table, &mut issues);
            continue;
        }
        blank_run = 0;
        if trimmed == PAGE_BREAK_MARK {
            finish_table(&mut table, &mut issues);
            out.push(page_break(target));
            continue;
        }
        if let Some(level) = heading_level(trimmed) {
            finish_table(&mut table, &mut issues);
            if last_heading > 0 && level > last_heading + 1 {
                push_issue(&mut issues, format!("标题跳级: 「{}」从 H{} 直接到 H{}", short(trimmed), last_heading, level));
            }
            last_heading = level;
            out.push(line);
            continue;
        }
        let mut line = rewrite_images(&line, base_dir, &mut issues);
        if trimmed.starts_with('|') {
            line = line.replace("<br/>", " ").replace("<br />", " ").replace("<br>", " ");
            track_table(&mut table, &line);
        } else {
            finish_table(&mut table, &mut issues);
            line = line.replace("<br/>", "\n").replace("<br />", "\n").replace("<br>", "\n");
            if let Some(tag) = RAW_HTML_TAGS.iter().find(|tag| trimmed.to_ascii_lowercase().starts_with(*tag)) {
                push_issue(&mut issues, format!("裸 HTML 会被丢弃: 「{}…」请改成 Markdown 结构", tag));
            }
        }
        out.push(line);
    }
    finish_table(&mut table, &mut issues);

    let mut markdown = out.join("\n");
    while markdown.ends_with("\n\n") {
        markdown.pop();
    }
    markdown.push('\n');
    Normalized { markdown, issues }
}

struct TableState {
    header: String,
    columns: usize,
    rows: usize,
    has_separator: bool,
    mismatch: bool,
}

fn track_table(table: &mut Option<TableState>, line: &str) {
    let columns = pipe_columns(line);
    match table {
        None => {
            *table = Some(TableState { header: line.trim().to_string(), columns, rows: 0, has_separator: false, mismatch: false });
        }
        Some(state) => {
            if !state.has_separator && is_separator(line) {
                state.has_separator = true;
                if columns != state.columns {
                    state.mismatch = true;
                }
                return;
            }
            state.rows += 1;
            if columns != state.columns {
                state.mismatch = true;
            }
        }
    }
}

fn finish_table(table: &mut Option<TableState>, issues: &mut Vec<String>) {
    let Some(state) = table.take() else {
        return;
    };
    let head = short(&state.header);
    if !state.has_separator {
        push_issue(issues, format!("表格缺少 | --- | 分隔行, 会被当成普通文字: 「{}」", head));
        return;
    }
    if state.mismatch {
        push_issue(issues, format!("表格各行竖线数不一致(表头 {} 列): 「{}」", state.columns, head));
    }
    if state.columns > TABLE_MAX_COLUMNS {
        push_issue(issues, format!("表格 {} 列超过 {} 列, 页宽放不下, 请拆表或改为分节描述: 「{}」", state.columns, TABLE_MAX_COLUMNS, head));
    }
    if state.rows == 0 {
        push_issue(issues, format!("表格只有表头没有数据行: 「{}」", head));
    }
}

fn pipe_columns(line: &str) -> usize {
    let trimmed = line.trim().trim_start_matches('|').trim_end_matches('|');
    let mut count = 1usize;
    let mut escaped = false;
    let mut in_code = false;
    for ch in trimmed.chars() {
        match ch {
            '\\' => escaped = !escaped,
            '`' if !escaped => in_code = !in_code,
            '|' if !escaped && !in_code => count += 1,
            _ => escaped = false,
        }
    }
    count
}

fn is_separator(line: &str) -> bool {
    let body = line.trim().trim_matches('|');
    !body.is_empty() && body.chars().all(|ch| matches!(ch, '-' | ':' | '|' | ' '))
}

fn heading_level(trimmed: &str) -> Option<usize> {
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if level == 0 || level > 6 || !trimmed[level..].starts_with(' ') {
        return None;
    }
    Some(level)
}

fn page_break(target: DocTarget) -> String {
    match target {
        DocTarget::Docx => "```{=openxml}\n<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>\n```".to_string(),
        DocTarget::Pdf => "```{=typst}\n#pagebreak()\n```".to_string(),
    }
}

fn rewrite_images(line: &str, base_dir: Option<&Path>, issues: &mut Vec<String>) -> String {
    if !line.contains("![") {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("![") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(open) = after.find("](") else {
            out.push_str(after);
            return out;
        };
        let Some(close) = after[open + 2..].find(')') else {
            out.push_str(after);
            return out;
        };
        let target_raw = &after[open + 2..open + 2 + close];
        let (target, title) = match target_raw.find(' ') {
            Some(space) => (&target_raw[..space], &target_raw[space..]),
            None => (target_raw, ""),
        };
        out.push_str(&after[..open + 2]);
        out.push_str(&resolve_image(target, base_dir, issues));
        out.push_str(title);
        out.push(')');
        rest = &after[open + 2 + close + 1..];
    }
    out.push_str(rest);
    out
}

fn resolve_image(target: &str, base_dir: Option<&Path>, issues: &mut Vec<String>) -> String {
    let trimmed = target.trim_matches(|ch| ch == '<' || ch == '>');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("data:") {
        return target.to_string();
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        if !path.exists() {
            push_issue(issues, format!("图片不存在: {}", trimmed));
        }
        return target.to_string();
    }
    match base_dir {
        Some(dir) => {
            let joined = dir.join(path);
            if !joined.exists() {
                push_issue(issues, format!("图片不存在: {}", joined.display()));
            }
            joined.to_string_lossy().replace('\\', "/")
        }
        None => {
            push_issue(issues, format!("图片用了相对路径但没有 source 可供解析, 请改为绝对路径: {}", trimmed));
            target.to_string()
        }
    }
}

fn push_issue(issues: &mut Vec<String>, issue: String) {
    if issues.len() < MAX_ISSUES && !issues.contains(&issue) {
        issues.push(issue);
    }
}

fn short(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= 40 {
        return text.to_string();
    }
    format!("{}…", chars[..40].iter().collect::<String>())
}
