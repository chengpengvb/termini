const REPEAT_TIMES: usize = 6;
const MIN_UNIT_CHARS: usize = 8;
const MAX_UNIT_CHARS: usize = 400;
const CHECK_EVERY_CHARS: usize = 64;
const WINDOW_CHARS: usize = MAX_UNIT_CHARS * REPEAT_TIMES + CHECK_EVERY_CHARS;

pub struct RepeatGuard {
    tail: Vec<char>,
    total_chars: usize,
    since_check: usize,
}

pub struct RepeatHit {
    pub keep_chars: usize,
    pub unit: String,
    pub times: usize,
}

impl Default for RepeatGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl RepeatGuard {
    pub fn new() -> Self {
        Self { tail: Vec::with_capacity(WINDOW_CHARS), total_chars: 0, since_check: 0 }
    }

    pub fn total_chars(&self) -> usize {
        self.total_chars
    }

    pub fn feed(&mut self, delta: &str) -> Option<RepeatHit> {
        for ch in delta.chars() {
            self.tail.push(ch);
            self.total_chars += 1;
            self.since_check += 1;
        }
        if self.tail.len() > WINDOW_CHARS {
            let drop = self.tail.len() - WINDOW_CHARS;
            self.tail.drain(..drop);
        }
        if self.since_check < CHECK_EVERY_CHARS {
            return None;
        }
        self.since_check = 0;
        self.detect()
    }

    fn detect(&self) -> Option<RepeatHit> {
        let len = self.tail.len();
        let max_unit = MAX_UNIT_CHARS.min(len / REPEAT_TIMES);
        for unit_len in MIN_UNIT_CHARS..=max_unit {
            if !self.tail_repeats(unit_len) {
                continue;
            }
            let times = self.count_repeats(unit_len);
            let unit: String = self.tail[len - unit_len..].iter().collect();
            if unit.trim().is_empty() {
                continue;
            }
            let keep_chars = self.total_chars.saturating_sub(unit_len * (times - 1));
            return Some(RepeatHit { keep_chars, unit, times });
        }
        None
    }

    fn tail_repeats(&self, unit_len: usize) -> bool {
        let len = self.tail.len();
        let unit = &self.tail[len - unit_len..];
        (1..REPEAT_TIMES).all(|index| {
            let end = len - unit_len * index;
            &self.tail[end - unit_len..end] == unit
        })
    }

    fn count_repeats(&self, unit_len: usize) -> usize {
        let len = self.tail.len();
        let unit = &self.tail[len - unit_len..];
        let mut times = 1usize;
        while (times + 1) * unit_len <= len {
            let end = len - unit_len * times;
            if &self.tail[end - unit_len..end] != unit {
                break;
            }
            times += 1;
        }
        times
    }
}

pub fn truncate_chars(text: &str, keep_chars: usize) -> String {
    text.chars().take(keep_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_all(guard: &mut RepeatGuard, text: &str) -> Option<RepeatHit> {
        let chars: Vec<char> = text.chars().collect();
        for chunk in chars.chunks(7) {
            let piece: String = chunk.iter().collect();
            if let Some(hit) = guard.feed(&piece) {
                return Some(hit);
            }
        }
        None
    }

    #[test]
    fn detects_sentence_loop() {
        let head = "第一段正常内容。\n";
        let unit = "六、歌詞を店に持っていることがある。";
        let full = format!("{}{}", head, unit.repeat(40));
        let mut guard = RepeatGuard::new();
        let hit = feed_all(&mut guard, &full).expect("expected a repeat to be detected");
        assert!(hit.times >= REPEAT_TIMES);
        assert!(unit.contains(hit.unit.trim()) || hit.unit.contains("歌詞"));
        let kept = truncate_chars(&full, hit.keep_chars);
        assert!(kept.starts_with(head));
        assert!(kept.chars().count() < full.chars().count());
    }

    #[test]
    fn ignores_normal_options() {
        let text = (1..=40).map(|index| format!("{}. 題目{}\nA. 甲\nB. 乙\nC. 丙\nD. 丁\n", index, index)).collect::<String>();
        let mut guard = RepeatGuard::new();
        assert!(feed_all(&mut guard, &text).is_none(), "正常选项不应判为重复");
    }
}
