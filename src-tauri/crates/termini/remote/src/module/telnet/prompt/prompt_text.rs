use regex::Regex;
use std::sync::OnceLock;

const BACKSPACE: char = '\u{8}';

pub const COMMAND_EOL: &str = "\r";

fn regex_huawei_cursor_left() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r" \x1b\[\d+D").expect("huawei cursor left regex"))
}

fn regex_ansi_osc() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\x1b][^\x07\x1b]*(?:\x07|\x1b\\)").expect("ansi osc regex"))
}

fn regex_ansi_csi() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\x1b\[[0-9;?]*[a-zA-Z]").expect("ansi csi regex"))
}

fn regex_ansi_simple() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\x1b(?:[()][0-9A-Za-z]|[=>EM78c])").expect("ansi simple regex"))
}

fn regex_line_feed() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\r\r\r\n|\r\r\n|\r\n|\n\r").expect("line feed regex"))
}

pub fn strip_ansi_escape_codes(input: &str) -> String {
    if !input.contains('\u{1b}') {
        return input.to_string();
    }
    let text = regex_huawei_cursor_left().replace_all(input, "");
    let text = regex_ansi_osc().replace_all(&text, "");
    let text = regex_ansi_csi().replace_all(&text, "");
    regex_ansi_simple().replace_all(&text, "").into_owned()
}

pub fn normalize_linefeeds(input: &str) -> String {
    let text = regex_line_feed().replace_all(input, "\n");
    text.replace('\r', "\n")
}

pub fn normalize_command(command: &str) -> String {
    format!("{}{}", command.trim_end(), COMMAND_EOL)
}

pub fn strip_backspaces(text: &str) -> String {
    if text.contains(BACKSPACE) {
        text.replace(BACKSPACE, "")
    } else {
        text.to_string()
    }
}

pub fn strip_command_echo(command: &str, output: &str) -> String {
    let output = strip_backspaces(output);
    let command = command.trim();
    if command.is_empty() {
        return output;
    }
    let trimmed = output.trim_start_matches(['\r', '\n']);
    if !trimmed.starts_with(command) {
        return output;
    }
    match trimmed.split_once('\n') {
        Some((_, rest)) => rest.to_string(),
        None => String::new(),
    }
}

pub fn trim_line_ends(text: &str) -> String {
    text.split('\n').map(|line| line.trim_end()).collect::<Vec<&str>>().join("\n")
}

pub fn strip_trailing_prompt(prompt_line: &str, output: &str) -> String {
    if prompt_line.is_empty() {
        return output.to_string();
    }
    let mut lines: Vec<&str> = output.split('\n').collect();
    while let Some(last) = lines.last() {
        if last.trim().is_empty() {
            lines.pop();
            continue;
        }
        if last.contains(prompt_line) {
            lines.pop();
        }
        break;
    }
    lines.join("\n")
}

pub fn extract_prompt_line(text: &str) -> Option<String> {
    let normalized = normalize_linefeeds(&strip_ansi_escape_codes(text));
    normalized.split('\n').rev().map(|line| line.trim()).find(|line| !line.is_empty()).map(|line| line.to_string())
}

pub fn extract_base_prompt(prompt_line: &str) -> String {
    let mut base = prompt_line.trim();
    if let Some(last) = base.chars().last() {
        if matches!(last, '#' | '>' | ']' | '$' | '%') {
            base = &base[..base.len() - last.len_utf8()];
        }
    }
    if let Some(first) = base.chars().next() {
        if matches!(first, '<' | '[') {
            base = &base[first.len_utf8()..];
        }
    }
    base.trim().to_string()
}
