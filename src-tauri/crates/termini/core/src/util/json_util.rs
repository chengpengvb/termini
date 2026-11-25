pub fn parse_string_array_prefix(raw: &str, keys: &[&str]) -> Option<Vec<String>> {
    let at = keys.iter().find_map(|key| raw.find(*key))?;
    let after = &raw[at..];
    let bracket = after.find('[')?;
    parse_json_string_array(&after[bracket + 1..])
}

fn parse_json_string_array(rest: &str) -> Option<Vec<String>> {
    let bytes = rest.as_bytes();
    let mut items = Vec::new();
    let mut i = 0;
    loop {
        while i < rest.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= rest.len() {
            return Some(items);
        }
        match bytes[i] {
            b']' => return Some(items),
            b',' => {
                i += 1;
                continue;
            }
            b'"' => match parse_json_string(&rest[i..]) {
                Some((value, used)) => {
                    items.push(value);
                    i += used;
                }
                None => return Some(items),
            },
            _ => {
                if items.is_empty() {
                    return None;
                }
                return Some(items);
            }
        }
    }
}

fn parse_json_string(input: &str) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut out = String::new();
    let mut i = 1;
    while i < input.len() {
        let c = bytes[i];
        if c == b'"' {
            return Some((out, i + 1));
        }
        if c == b'\\' {
            let (ch, used) = parse_json_escape(&input[i..])?;
            out.push(ch);
            i += used;
            continue;
        }
        if c < 0x20 {
            return None;
        }
        let ch = input[i..].chars().next()?;
        out.push(ch);
        i += ch.len_utf8();
    }
    None
}

fn parse_json_escape(rest: &str) -> Option<(char, usize)> {
    let bytes = rest.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'\\' {
        return None;
    }
    match bytes[1] {
        b'"' => Some(('"', 2)),
        b'\\' => Some(('\\', 2)),
        b'/' => Some(('/', 2)),
        b'n' => Some(('\n', 2)),
        b'r' => Some(('\r', 2)),
        b't' => Some(('\t', 2)),
        b'b' => Some(('\u{0008}', 2)),
        b'f' => Some(('\u{000c}', 2)),
        b'u' => {
            if rest.len() < 6 {
                return None;
            }
            let code = u32::from_str_radix(&rest[2..6], 16).ok()?;
            Some((char::from_u32(code)?, 6))
        }
        _ => None,
    }
}
