pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(value) = chars.next() {
        if value == '\u{1b}' {
            skip_escape_sequence(&mut chars);
            continue;
        }
        if value == '\n' || value == '\t' {
            out.push(value);
            continue;
        }
        if value.is_control() {
            continue;
        }
        out.push(value);
    }
    out
}

fn skip_escape_sequence(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    let Some(&next) = chars.peek() else {
        return;
    };
    if next == '[' {
        chars.next();
        while let Some(value) = chars.next() {
            if ('\u{40}'..='\u{7e}').contains(&value) {
                return;
            }
        }
        return;
    }
    if next == ']' {
        chars.next();
        while let Some(value) = chars.next() {
            if value == '\u{7}' {
                return;
            }
            if value == '\u{1b}' {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                return;
            }
        }
        return;
    }
    chars.next();
}
