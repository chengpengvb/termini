use std::sync::OnceLock;
use rhai::{Dynamic, Engine, Scope};

pub const VALUE_NAME: &str = "value";

const MAX_EXPR_DEPTH: usize = 32;
const MAX_OPERATIONS: u64 = 10_000;
const MAX_STRING_SIZE: usize = 64 * 1024;

static ENGINE: OnceLock<Engine> = OnceLock::new();

fn engine() -> &'static Engine {
    ENGINE.get_or_init(|| {
        let mut engine = Engine::new();
        engine.set_max_operations(MAX_OPERATIONS);
        engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_EXPR_DEPTH);
        engine.set_max_string_size(MAX_STRING_SIZE);
        engine
    })
}

pub fn check(transform_expr: &str) -> anyhow::Result<()> {
    if transform_expr.trim().is_empty() {
        return Ok(());
    }
    engine().compile(transform_expr)?;
    Ok(())
}

pub fn transform(transform_expr: &str, raw: &str) -> anyhow::Result<String> {
    if transform_expr.trim().is_empty() {
        return Ok(raw.to_string());
    }
    let mut scope = Scope::new();
    scope.push(VALUE_NAME, raw.to_string());
    let result: Dynamic = engine().eval_with_scope(&mut scope, transform_expr)?;
    to_number_text(result)
}

fn to_number_text(result: Dynamic) -> anyhow::Result<String> {
    if let Ok(value) = result.as_int() {
        return Ok(value.to_string());
    }
    if let Ok(value) = result.as_float() {
        return Ok(format!("{}", value));
    }
    if let Ok(value) = result.as_bool() {
        return Ok(if value { "1".to_string() } else { "0".to_string() });
    }
    let text = result.into_string().map_err(|kind| anyhow::anyhow!("script returned a {} value that cannot be converted to a number", kind))?;
    let text = text.trim().to_string();
    if text.parse::<f64>().is_err() {
        anyhow::bail!("script returned \"{}\", which is not a number", text);
    }
    Ok(text)
}
