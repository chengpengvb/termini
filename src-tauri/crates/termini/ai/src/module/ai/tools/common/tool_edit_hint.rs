const MAX_HINT_LINES: usize = 3;
const HINT_LINE_MAX_CHARS: usize = 120;

pub fn not_found_hint(content: &str, old_string: &str) -> String {
    if confusable_only_mismatch(content, old_string) {
        return "; the difference seems limited to confusable characters (quotes/dashes/spaces); old_string must be copied character by character from the file, re-run file_read to verify".to_string();
    }
    let needle = first_meaningful_line(old_string);
    if let Some(needle) = needle {
        let matched = similar_lines(content, &needle);
        if !matched.is_empty() {
            let listed: Vec<String> = matched.iter().map(|(number, line)| format!("line {}: {}", number, truncate_line(line))).collect();
            return format!("; similar content found at {}; the file may have changed since it was read, re-run file_read and retry with the exact text", listed.join(", "));
        }
    }
    "; re-run file_read to get the current content and retry".to_string()
}

pub fn match_line_numbers(content: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut numbers = Vec::new();
    for (offset, _) in content.match_indices(needle) {
        let line = content[..offset].matches('\n').count() + 1;
        numbers.push(line);
    }
    numbers
}

pub fn format_line_numbers(numbers: &[usize]) -> String {
    let listed: Vec<String> = numbers.iter().take(MAX_HINT_LINES).map(|number| number.to_string()).collect();
    if numbers.len() > MAX_HINT_LINES {
        return format!("{} etc.", listed.join(", "));
    }
    listed.join(", ")
}

fn confusable_only_mismatch(content: &str, old_string: &str) -> bool {
    let normalized_old = normalize_confusables(old_string);
    if normalized_old == old_string && normalize_confusables(content) == content {
        return false;
    }
    normalize_confusables(content).contains(&normalized_old)
}

fn normalize_confusables(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' | '\u{00b4}' | '\u{2032}' => '\'',
            '\u{201c}' | '\u{201d}' | '\u{2033}' => '"',
            '\u{2010}' | '\u{2011}' | '\u{2013}' | '\u{2014}' | '\u{2212}' => '-',
            '\u{00a0}' | '\u{2007}' | '\u{202f}' | '\u{3000}' => ' ',
            other => other,
        })
        .collect()
}

fn first_meaningful_line(old_string: &str) -> Option<String> {
    old_string.lines().map(str::trim).find(|line| line.len() >= 4).map(|line| line.to_string())
}

fn similar_lines(content: &str, needle: &str) -> Vec<(usize, String)> {
    let mut matched = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim() != needle && !line.contains(needle) {
            continue;
        }
        matched.push((index + 1, line.trim().to_string()));
        if matched.len() >= MAX_HINT_LINES {
            break;
        }
    }
    matched
}

fn truncate_line(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= HINT_LINE_MAX_CHARS {
        return line.to_string();
    }
    let kept: String = chars.into_iter().take(HINT_LINE_MAX_CHARS).collect();
    format!("{}…", kept)
}
