use regex::Regex;
use std::sync::OnceLock;

const DROP_TAGS: &[&str] = &["script", "style", "noscript", "svg", "template", "iframe", "head"];
const BLOCK_TAGS: &[&str] = &[
    "p",
    "div",
    "br",
    "li",
    "ul",
    "ol",
    "tr",
    "td",
    "th",
    "table",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "section",
    "article",
    "header",
    "footer",
    "nav",
    "main",
    "aside",
    "blockquote",
    "pre",
    "hr",
    "dd",
    "dt",
    "dl",
    "figure",
    "figcaption",
    "summary",
    "details",
];

fn regex(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("html_text regex"))
}

pub fn looks_like_html(body: &str) -> bool {
    let head: String = body.trim_start().chars().take(512).collect::<String>().to_ascii_lowercase();
    head.starts_with("<!doctype html") || head.starts_with("<html") || head.contains("<head") || head.contains("<body")
}

pub fn html_to_text(html: &str) -> String {
    static DROP: OnceLock<Vec<Regex>> = OnceLock::new();
    static COMMENT: OnceLock<Regex> = OnceLock::new();
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    static TAG: OnceLock<Regex> = OnceLock::new();
    static SPACES: OnceLock<Regex> = OnceLock::new();
    static BLANK_LINES: OnceLock<Regex> = OnceLock::new();

    let title = extract_title(html);
    let mut text = regex(&COMMENT, r"(?s)<!--.*?-->").replace_all(html, "").into_owned();
    // regex crate 不支持反向引用, 每种要整段丢弃的标签各编一条
    let drop_patterns = DROP.get_or_init(|| DROP_TAGS.iter().map(|tag| Regex::new(&format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}\s*>")).expect("html_text drop regex")).collect::<Vec<Regex>>());
    for pattern in drop_patterns {
        text = pattern.replace_all(&text, "").into_owned();
    }
    let block_pattern = format!(r"(?i)</?({})\b[^>]*>", BLOCK_TAGS.join("|"));
    text = regex(&BLOCK, &block_pattern).replace_all(&text, "\n").into_owned();
    text = regex(&TAG, r"(?s)<[^>]+>").replace_all(&text, " ").into_owned();
    text = decode_entities(&text);
    text = regex(&SPACES, r"[ \t\r\f\v\u{a0}]+").replace_all(&text, " ").into_owned();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let joined = lines.join("\n");
    let collapsed = regex(&BLANK_LINES, r"\n{2,}").replace_all(&joined, "\n").into_owned();
    let body = collapsed.trim().to_string();
    match title {
        Some(title) if !body.starts_with(&title) => format!("# {}\n{}", title, body),
        _ => body,
    }
}

fn extract_title(html: &str) -> Option<String> {
    static TITLE: OnceLock<Regex> = OnceLock::new();
    let caps = regex(&TITLE, r"(?is)<title[^>]*>(.*?)</title>").captures(html)?;
    let raw = decode_entities(caps.get(1)?.as_str());
    let title = raw.split_whitespace().collect::<Vec<&str>>().join(" ");
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

fn decode_entities(text: &str) -> String {
    static NUMERIC: OnceLock<Regex> = OnceLock::new();
    let named = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&ldquo;", "“")
        .replace("&rdquo;", "”")
        .replace("&hellip;", "…")
        .replace("&mdash;", "—")
        .replace("&middot;", "·")
        .replace("&copy;", "©");
    let decoded = regex(&NUMERIC, r"&#(x[0-9a-fA-F]+|[0-9]+);").replace_all(&named, |caps: &regex::Captures| {
        let raw = &caps[1];
        let code = if let Some(hex) = raw.strip_prefix('x') { u32::from_str_radix(hex, 16).ok() } else { raw.parse::<u32>().ok() };
        code.and_then(char::from_u32).map(|ch| ch.to_string()).unwrap_or_default()
    });
    decoded.replace("&amp;", "&")
}
