const FENCE_MIN: usize = 3;
const FENCE_CHAR: char = '`';
const FENCE_LANG: &str = "text";

pub fn compose(text: &str) -> String {
    fenced(text.trim_matches('\n'))
}

pub fn compose_pages(pages: &[(String, String)]) -> String {
    pages.iter().map(|(label, text)| fenced_page(label, text)).collect::<Vec<String>>().join("\n")
}

pub fn fenced_page(label: &str, text: &str) -> String {
    format!("{}\n{}\n", label, fenced(text.trim_matches('\n')))
}

pub fn fenced(text: &str) -> String {
    let fence: String = std::iter::repeat(FENCE_CHAR).take(fence_len(text)).collect();
    format!("{}{}\n{}\n{}", fence, FENCE_LANG, text, fence)
}

fn fence_len(text: &str) -> usize {
    let mut longest = 0usize;
    let mut run = 0usize;
    for ch in text.chars() {
        if ch == FENCE_CHAR {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    FENCE_MIN.max(longest + 1)
}

pub fn tail_chars(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    text.chars().skip(count - max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_keeps_lines_verbatim() {
        assert_eq!(compose("    缩进行\n# 井号开头\n1. 序号\n\n> 引用"), "```text\n    缩进行\n# 井号开头\n1. 序号\n\n> 引用\n```");
    }

    #[test]
    fn fence_outgrows_backticks_in_content() {
        let out = fenced("a\n```\ncode\n```\nb");
        assert!(out.starts_with("````text\n"));
        assert!(out.ends_with("\n````"));
    }

    #[test]
    fn pages_are_labelled_and_fenced() {
        let out = compose_pages(&[("--- page 1 ---".to_string(), "甲".to_string()), ("--- page 2 ---".to_string(), "乙".to_string())]);
        assert_eq!(out, "--- page 1 ---\n```text\n甲\n```\n\n--- page 2 ---\n```text\n乙\n```\n");
    }
}
