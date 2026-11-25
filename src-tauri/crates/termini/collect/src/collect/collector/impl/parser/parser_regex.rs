use regex::Regex;

pub const CAPTURE_NAME: &str = "value";

pub fn parse(parse_expr: &str, output: &str) -> anyhow::Result<Option<String>> {
    if parse_expr.trim().is_empty() {
        anyhow::bail!("regex expression is empty");
    }
    let regex = Regex::new(parse_expr)?;
    let Some(captures) = regex.captures(output) else {
        return Ok(None);
    };
    let matched = captures.name(CAPTURE_NAME).or_else(|| captures.get(1)).or_else(|| captures.get(0));
    Ok(matched.map(|item| item.as_str().trim().to_string()))
}
