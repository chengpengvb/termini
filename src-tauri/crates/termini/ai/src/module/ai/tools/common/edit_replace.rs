use crate::module::ai::tools::common::tool_edit_hint::{format_line_numbers, match_line_numbers, not_found_hint};

pub struct ReplaceRequest<'a> {
    pub old_string: &'a str,
    pub new_string: &'a str,
    pub replace_all: bool,
}

fn file_eol(raw: &str) -> &'static str {
    match raw.find('\n') {
        Some(index) if raw[..index].ends_with('\r') => "\r\n",
        _ => "\n",
    }
}

fn normalize_eol(raw: &str) -> (String, Vec<usize>) {
    let mut text = String::with_capacity(raw.len());
    let mut offsets = Vec::with_capacity(raw.len() + 1);
    let mut cursor = raw.char_indices().peekable();
    while let Some((index, value)) = cursor.next() {
        if value == '\r' && cursor.peek().is_some_and(|(_, next)| *next == '\n') {
            cursor.next();
            offsets.push(index);
            text.push('\n');
            continue;
        }
        for step in 0..value.len_utf8() {
            offsets.push(index + step);
        }
        text.push(value);
    }
    offsets.push(raw.len());
    (text, offsets)
}

pub fn first_match_line(raw: &str, old_string: &str) -> Option<usize> {
    if old_string.is_empty() {
        return None;
    }
    let body = raw.strip_prefix('\u{FEFF}').unwrap_or(raw);
    let (text, _) = normalize_eol(body);
    let needle = old_string.replace("\r\n", "\n");
    let start = text.find(&needle)?;
    Some(text[..start].matches('\n').count() + 1)
}

/// 应用一次替换, 返回新的完整原文与实际替换处数。BOM 与原有换行风格保持不变。
pub fn apply_one(raw: &str, label: &str, request: &ReplaceRequest) -> Result<(String, usize), String> {
    let (bom, body) = match raw.strip_prefix('\u{FEFF}') {
        Some(rest) => ("\u{FEFF}", rest),
        None => ("", raw),
    };
    let (text, offsets) = normalize_eol(body);
    let needle = request.old_string.replace("\r\n", "\n");
    let starts: Vec<usize> = text.match_indices(&needle).map(|(start, _)| start).collect();
    if starts.is_empty() {
        return Err(format!("old_string not found in {}{}", label, not_found_hint(&text, &needle)));
    }
    if starts.len() > 1 && !request.replace_all {
        let numbers = match_line_numbers(&text, &needle);
        return Err(format!("old_string matched {} times in {} (lines {}); make it unique or set replace_all=true", starts.len(), label, format_line_numbers(&numbers)));
    }
    let replacement = request.new_string.replace("\r\n", "\n").replace('\n', file_eol(body));
    let targets = if request.replace_all { &starts[..] } else { &starts[..1] };
    let mut next = String::with_capacity(raw.len());
    next.push_str(bom);
    let mut cursor = 0;
    for start in targets {
        next.push_str(&body[cursor..offsets[*start]]);
        next.push_str(&replacement);
        cursor = offsets[start + needle.len()];
    }
    next.push_str(&body[cursor..]);
    Ok((next, targets.len()))
}

/// 按顺序把多次替换应用到同一份原文上。
///
/// 逐次基于上一步的结果继续, 所以同一个文件里的多处改动可以在一轮里一起提交,
/// 不会因为前一处改动使后一处的 old_string 失配。任何一处失败即整体失败, 调用方不落盘。
pub fn apply_all(raw: &str, label: &str, requests: &[ReplaceRequest]) -> Result<(String, usize), String> {
    if requests.is_empty() {
        return Err(format!("no edits for {}", label));
    }
    let mut current = raw.to_string();
    let mut applied = 0usize;
    for (index, request) in requests.iter().enumerate() {
        let (next, count) = apply_one(&current, label, request).map_err(|error| if requests.len() == 1 { error } else { format!("edit #{} failed: {}", index + 1, error) })?;
        current = next;
        applied += count;
    }
    Ok((current, applied))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(old: &'a str, new: &'a str, all: bool) -> ReplaceRequest<'a> {
        ReplaceRequest { old_string: old, new_string: new, replace_all: all }
    }

    #[test]
    fn applies_edits_in_order_on_one_file() {
        let raw = "let a = 1;\nlet b = 2;\n";
        let edits = vec![request("let a = 1;", "let a = 10;", false), request("let b = 2;", "let b = 20;", false)];
        let (text, applied) = apply_all(raw, "x.rs", &edits).expect("apply");
        assert_eq!(applied, 2);
        assert_eq!(text, "let a = 10;\nlet b = 20;\n");
    }

    #[test]
    fn later_edit_sees_earlier_result() {
        let raw = "value = 1;\n";
        // 第二处的 old_string 只有在第一处生效后才存在
        let edits = vec![request("value = 1;", "value = 2;", false), request("value = 2;", "value = 3;", false)];
        let (text, _) = apply_all(raw, "x.rs", &edits).expect("apply");
        assert_eq!(text, "value = 3;\n");
    }

    #[test]
    fn any_failure_aborts_whole_batch() {
        let raw = "keep = 1;\n";
        let edits = vec![request("keep = 1;", "keep = 2;", false), request("missing_zzz", "whatever", false)];
        let error = apply_all(raw, "x.rs", &edits).expect_err("must fail");
        assert!(error.contains("edit #2 failed"), "要指明是第几处失败: {error}");
    }

    #[test]
    fn rejects_ambiguous_match_without_replace_all() {
        let raw = "dup;\ndup;\n";
        let error = apply_one(raw, "x.rs", &request("dup;", "one;", false)).expect_err("must fail");
        assert!(error.contains("matched 2 times"), "{error}");
        let (text, applied) = apply_one(raw, "x.rs", &request("dup;", "one;", true)).expect("replace_all");
        assert_eq!(applied, 2);
        assert_eq!(text, "one;\none;\n");
    }

    #[test]
    fn preserves_crlf_and_bom() {
        let raw = "\u{FEFF}line1\r\nline2\r\n";
        let (text, _) = apply_one(raw, "x.rs", &request("line2", "line2b", false)).expect("apply");
        assert!(text.starts_with('\u{FEFF}'), "BOM 必须保留");
        assert!(text.contains("line2b\r\n"), "原有 CRLF 必须保留: {:?}", text);
        assert!(!text.contains("line2b\n\r"), "换行不能被打乱: {:?}", text);
    }

    #[test]
    fn reports_real_line_number_of_match() {
        let raw = "a\nb\nTARGET\nd\n";
        assert_eq!(first_match_line(raw, "TARGET"), Some(3));
        assert_eq!(first_match_line(raw, "a"), Some(1));
        assert_eq!(first_match_line(raw, "missing"), None);
        assert_eq!(first_match_line(raw, ""), None);
    }

    #[test]
    fn line_number_is_not_skewed_by_crlf_or_bom() {
        let raw = "\u{FEFF}a\r\nb\r\nTARGET\r\n";
        // 归一化后按 LF 计数, CRLF 文件也要拿到第 3 行
        assert_eq!(first_match_line(raw, "TARGET"), Some(3));
    }

    #[test]
    fn old_string_may_use_crlf_against_lf_file() {
        let raw = "a\nb\n";
        let (text, _) = apply_one(raw, "x.rs", &request("a\r\nb", "x\r\ny", false)).expect("apply");
        assert_eq!(text, "x\ny\n");
    }
}
