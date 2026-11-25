const MAX_OVERLAP_ROWS: usize = 12;
const MIN_SINGLE_ROW_CHARS: usize = 4;

pub fn clean(raw: &str) -> String {
    let rows: Vec<&str> = raw.lines().map(str::trim_end).collect();
    let mut out: Vec<&str> = Vec::with_capacity(rows.len());
    for row in rows {
        if row.trim().is_empty() && out.last().map(|last: &&str| last.trim().is_empty()).unwrap_or(true) {
            continue;
        }
        out.push(row);
    }
    while out.last().map(|last| last.trim().is_empty()).unwrap_or(false) {
        out.pop();
    }
    out.join("\n")
}

pub fn stitch(parts: &[String]) -> String {
    let mut rows: Vec<String> = Vec::new();
    for part in parts {
        let next: Vec<&str> = part.lines().collect();
        if rows.is_empty() {
            rows.extend(next.iter().map(|line| line.to_string()));
            continue;
        }
        if let Some(cut) = overlap_start(&rows, &next) {
            rows.truncate(cut);
        }
        rows.extend(next.iter().map(|line| line.to_string()));
    }
    clean(&rows.join("\n"))
}

fn overlap_start(rows: &[String], next: &[&str]) -> Option<usize> {
    let tail: Vec<(usize, String)> = filled_tail(rows);
    let head: Vec<String> = next.iter().filter(|line| !line.trim().is_empty()).take(MAX_OVERLAP_ROWS).map(|line| normalize(line)).collect();
    if tail.is_empty() || head.is_empty() {
        return None;
    }
    let mut best: Option<(usize, usize)> = None;
    for start in 0..tail.len() {
        let mut matched = 0usize;
        while start + matched < tail.len() && matched < head.len() && tail[start + matched].1 == head[matched] {
            matched += 1;
        }
        let strong = matched >= 2 || (matched == 1 && head[0].chars().count() >= MIN_SINGLE_ROW_CHARS);
        if strong && best.map(|(_, count)| matched > count).unwrap_or(true) {
            best = Some((tail[start].0, matched));
        }
    }
    best.map(|(index, _)| index)
}

fn filled_tail(rows: &[String]) -> Vec<(usize, String)> {
    let picked: Vec<(usize, String)> = rows.iter().enumerate().rev().filter(|(_, line)| !line.trim().is_empty()).take(MAX_OVERLAP_ROWS).map(|(index, line)| (index, normalize(line))).collect();
    picked.into_iter().rev().collect()
}

fn normalize(line: &str) -> String {
    line.chars().filter(|ch| ch.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_trims_trailing_space_and_collapses_blank_runs() {
        assert_eq!(clean("甲  \n\n\n乙\n\n"), "甲\n\n乙");
    }

    #[test]
    fn stitch_drops_duplicated_boundary_rows_ignoring_punctuation() {
        let previous = "9. いつ友達が来ますか。\n听下面5段录音，每段录音后有几个小题，从A、B、C三个选项中选出最佳选项。每段录\n音后，你将有时间阅读各个小题。".to_string();
        let next = "听下面5段录音,每段录音后有几个小题,从A、B、C三个选项中选出最佳选项。每段录\n音后,你将有时间阅读各个小题。\n每段录音播放两遍。\n10. 旅行".to_string();
        assert_eq!(
            stitch(&[previous, next]),
            "9. いつ友達が来ますか。\n听下面5段录音,每段录音后有几个小题,从A、B、C三个选项中选出最佳选项。每段录\n音后,你将有时间阅读各个小题。\n每段录音播放两遍。\n10. 旅行"
        );
        assert_eq!(stitch(&["甲".to_string(), "乙".to_string()]), "甲\n乙");
    }

    #[test]
    fn stitch_replaces_truncated_tail_row_with_the_next_tile_version() {
        let previous = "第二部分 阅读理解\n（一）\n冬のある日, 狐が他人の家から魚を盗みました。".to_string();
        let next = "第二部分 阅读理解\n（一）\n冬のある日, 熊が狐に会いました。狐は他人の家から魚を盗みました。\n「どこで」".to_string();
        assert_eq!(stitch(&[previous, next]), "第二部分 阅读理解\n（一）\n冬のある日, 熊が狐に会いました。狐は他人の家から魚を盗みました。\n「どこで」");
        let footer_prev = "20. 女の人は今どんな気持ちですか。\n第 7 页 共 8 页".to_string();
        let footer_next = "第 7 页 共 8 页\nでも、日本では".to_string();
        assert_eq!(stitch(&[footer_prev, footer_next]), "20. 女の人は今どんな気持ちですか。\n第 7 页 共 8 页\nでも、日本では");
    }
}
