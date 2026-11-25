use summer::App;
use anyhow::{bail, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};
use termini_core::model::base_target::{AiFieldMeta, BaseTarget};

pub const LIST_LIMIT_MAX: usize = 20;
pub const LIST_LIMIT_DEFAULT: usize = 8;

#[async_trait]
pub trait AiCrud: Send + Sync {
    fn entity(&self) -> &'static str;
    fn notify_entity_type(&self) -> i32;
    fn fields(&self) -> &'static [AiFieldMeta];
    async fn get(&self, id: String) -> Result<Value>;
    async fn insert(&self, row: Value) -> Result<Value>;
    async fn update(&self, row: Value) -> Result<Value>;
    async fn delete(&self, id: String) -> Result<Value>;
    async fn insert_many(&self, rows: Vec<Value>) -> Result<Value>;
    async fn delete_many(&self, ids: Vec<String>) -> Result<Value>;
    async fn list(&self, offset: usize, limit: usize, keyword: Option<String>) -> Result<Value>;
}

pub struct AiCrudEntry {
    pub fetch: fn(&App) -> Option<Arc<dyn AiCrud>>,
}

inventory::collect!(AiCrudEntry);

static HANDLERS: OnceLock<Vec<Arc<dyn AiCrud>>> = OnceLock::new();

pub fn wire(app: &App) {
    let mut items: Vec<Arc<dyn AiCrud>> = Vec::new();
    for reg in inventory::iter::<AiCrudEntry> {
        items.push((reg.fetch)(app).expect("ai crud service missing"));
    }
    items.sort_by_key(|item| item.entity());
    for pair in items.windows(2) {
        if pair[0].entity() == pair[1].entity() {
            panic!("duplicate ai crud entity {}", pair[0].entity());
        }
    }
    if HANDLERS.set(items).is_err() {
        panic!("ai crud already wired");
    }
}

pub fn handlers() -> Result<&'static [Arc<dyn AiCrud>]> {
    match HANDLERS.get() {
        Some(items) => Ok(items.as_slice()),
        None => bail!("ai crud is not wired"),
    }
}

pub fn handler(name: &str) -> Result<Arc<dyn AiCrud>> {
    let key = name.trim();
    if key.is_empty() {
        bail!("entity is empty");
    }
    handlers()?.iter().find(|item| item.entity() == key).cloned().ok_or_else(|| anyhow::anyhow!("unknown entity: {}", key))
}

pub fn hidden_names<T: BaseTarget>() -> Vec<&'static str> {
    T::ai_fields().iter().filter(|field| field.hidden).map(|field| field.name).collect()
}

pub fn to_ai_row<T>(model: T) -> Result<Value>
where
    T: BaseTarget + serde::Serialize,
{
    let hidden = hidden_names::<T>();
    let model = model.for_ai();
    let mut value = serde_json::to_value(model)?;
    if let Some(obj) = value.as_object_mut() {
        for name in hidden {
            obj.insert(name.to_string(), Value::Null);
        }
    }
    Ok(value)
}

pub fn merge_row(mut current: Value, patch: Value, hidden: &[&str]) -> Result<Value> {
    let Some(patch_obj) = patch.as_object() else {
        bail!("row must be an object");
    };
    let Some(current_obj) = current.as_object_mut() else {
        bail!("current row is not an object");
    };
    for (key, value) in patch_obj {
        if value.is_null() && hidden.iter().any(|name| *name == key.as_str()) {
            continue;
        }
        current_obj.insert(key.clone(), value.clone());
    }
    Ok(current)
}

pub fn page_values(mut rows: Vec<Value>, offset: usize, limit: usize, keyword: Option<&str>) -> Value {
    rows.sort_by(|left, right| left.get("id").and_then(|value| value.as_str()).cmp(&right.get("id").and_then(|value| value.as_str())));
    let filtered: Vec<Value> = match keyword {
        Some(query) if !query.trim().is_empty() => {
            let tokens: Vec<String> = query.split_whitespace().map(|token| token.to_lowercase()).collect();
            rows.into_iter()
                .filter(|row| {
                    let text = row.to_string().to_lowercase();
                    tokens.iter().any(|token| text.contains(token))
                })
                .collect()
        }
        _ => rows,
    };
    let total = filtered.len();
    let limit = limit.clamp(1, LIST_LIMIT_MAX);
    let rows: Vec<Value> = filtered.into_iter().skip(offset).take(limit).collect();
    json!({ "total": total, "offset": offset, "limit": limit, "rows": rows })
}

pub fn take_optional_string(row: &mut Value, key: &str) -> Result<Option<String>> {
    let Some(obj) = row.as_object_mut() else {
        bail!("row must be an object");
    };
    let Some(value) = obj.remove(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(text) = value.as_str() else {
        bail!("field {} must be a string", key);
    };
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(text.to_string()))
}

pub fn page_entities(offset: usize, limit: usize, keyword: Option<&str>) -> Result<Value> {
    let mut rows: Vec<Value> = Vec::new();
    for item in handlers()? {
        rows.push(json!({
            "entity": item.entity(),
            "fields": item.fields().len(),
        }));
    }
    Ok(page_values(rows, offset, limit, keyword))
}
