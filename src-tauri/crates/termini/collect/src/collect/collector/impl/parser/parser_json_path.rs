use serde_json::Value;
use serde_json_path::JsonPath;

pub fn parse(parse_expr: &str, output: &str) -> anyhow::Result<Option<String>> {
    if parse_expr.trim().is_empty() {
        anyhow::bail!("JSONPath expression is empty");
    }
    let json: Value = serde_json::from_str(output.trim())?;
    let path = JsonPath::parse(parse_expr)?;
    let Some(node) = path.query(&json).first() else {
        return Ok(None);
    };
    Ok(Some(to_text(node)))
}

fn to_text(node: &Value) -> String {
    match node {
        Value::String(text) => text.trim().to_string(),
        other => other.to_string(),
    }
}
