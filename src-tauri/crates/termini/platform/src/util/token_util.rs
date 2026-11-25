const PER_MILLE: usize = 1000;
const CLAMP_HEAD_PERCENT: usize = 60;
const CLAMP_TAIL_PERCENT: usize = 40;
const ASCII_PER_MILLE: usize = 250;
const LATIN_PER_MILLE: usize = 330;
const GREEK_CYRILLIC_PER_MILLE: usize = 500;
const SEMITIC_PER_MILLE: usize = 600;
const INDIC_PER_MILLE: usize = 800;
const KANA_PER_MILLE: usize = 700;
const HANGUL_PER_MILLE: usize = 1200;
const CJK_IDEOGRAPH_PER_MILLE: usize = 1000;
const CJK_PUNCTUATION_PER_MILLE: usize = 800;
const PICTOGRAPH_PER_MILLE: usize = 2000;
const OTHER_PER_MILLE: usize = 500;

pub fn estimate_text_tokens(text: &str) -> usize {
    text.chars().map(char_per_mille).sum::<usize>() / PER_MILLE
}

pub fn take_text_tokens(text: &str, budget: usize) -> String {
    let mut used = 0usize;
    let mut out = String::new();
    for value in text.chars() {
        let next = used + char_per_mille(value);
        if next > budget.saturating_mul(PER_MILLE) {
            break;
        }
        used = next;
        out.push(value);
    }
    out
}

pub fn take_text_tokens_tail(text: &str, budget: usize) -> String {
    let mut used = 0usize;
    let mut picked: Vec<char> = Vec::new();
    for value in text.chars().rev() {
        let next = used + char_per_mille(value);
        if next > budget.saturating_mul(PER_MILLE) {
            break;
        }
        used = next;
        picked.push(value);
    }
    picked.reverse();
    picked.into_iter().collect()
}

pub fn clamp_text_tokens(text: String, max_tokens: usize) -> String {
    let tokens = estimate_text_tokens(&text);
    if tokens <= max_tokens {
        return text;
    }
    let head = take_text_tokens(&text, max_tokens * CLAMP_HEAD_PERCENT / 100);
    let tail = take_text_tokens_tail(&text, max_tokens * CLAMP_TAIL_PERCENT / 100);
    let omitted = tokens.saturating_sub(estimate_text_tokens(&head)).saturating_sub(estimate_text_tokens(&tail));
    format!("{}\n… {} of {} tokens omitted …\n{}", head, omitted, tokens, tail)
}

pub struct FittedSections {
    pub text: String,
    pub clamped: Vec<usize>,
}

pub fn fit_sections_to_budget(sections: Vec<String>, budget: usize) -> FittedSections {
    let costs: Vec<usize> = sections.iter().map(|section| estimate_text_tokens(section)).collect();
    if costs.iter().sum::<usize>() <= budget {
        return FittedSections { text: sections.concat(), clamped: Vec::new() };
    }
    let mut text = String::new();
    let mut clamped = Vec::new();
    let mut remaining = budget;
    let mut left = sections.len();
    for (index, (section, cost)) in sections.into_iter().zip(costs).enumerate() {
        let share = if left > 1 { remaining / left } else { remaining };
        if cost <= share {
            remaining = remaining.saturating_sub(cost);
            text.push_str(&section);
        } else {
            text.push_str(&clamp_text_tokens(section, share));
            text.push('\n');
            remaining = remaining.saturating_sub(share);
            clamped.push(index);
        }
        left = left.saturating_sub(1);
    }
    FittedSections { text, clamped }
}

fn char_per_mille(value: char) -> usize {
    match value as u32 {
        0x0000..=0x007F => ASCII_PER_MILLE,
        0x0080..=0x024F => LATIN_PER_MILLE,
        0x0370..=0x04FF => GREEK_CYRILLIC_PER_MILLE,
        0x0590..=0x06FF => SEMITIC_PER_MILLE,
        0x0900..=0x097F => INDIC_PER_MILLE,
        0x0E00..=0x0E7F => INDIC_PER_MILLE,
        0x1100..=0x11FF => HANGUL_PER_MILLE,
        0x3000..=0x303F => CJK_PUNCTUATION_PER_MILLE,
        0x3040..=0x30FF => KANA_PER_MILLE,
        0x3400..=0x4DBF => CJK_IDEOGRAPH_PER_MILLE,
        0x4E00..=0x9FFF => CJK_IDEOGRAPH_PER_MILLE,
        0xA960..=0xA97F => HANGUL_PER_MILLE,
        0xAC00..=0xD7FF => HANGUL_PER_MILLE,
        0xF900..=0xFAFF => CJK_IDEOGRAPH_PER_MILLE,
        0xFF00..=0xFFEF => CJK_PUNCTUATION_PER_MILLE,
        0x1F000..=0x1FBFF => PICTOGRAPH_PER_MILLE,
        0x20000..=0x3FFFF => CJK_IDEOGRAPH_PER_MILLE,
        _ => OTHER_PER_MILLE,
    }
}
