use summer::App;
use serde_json::Value;
use summer::tracing::warn;
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};
use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};
use crate::service::service_base::BaseService;
use termini_core::model::base_target::BaseTarget;
use termini_core::model::resp::resp_model::{BizError, ResultCode};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, PrimaryKeyTrait};

pub const OP_UPLOAD: i8 = 1;
pub const OP_DOWNLOAD: i8 = 2;

#[async_trait]
pub trait BaseSync: Send + Sync {
    fn sync_type(&self) -> i8;

    fn skip_id(&self) -> Option<&'static str> {
        None
    }

    fn merged(&self, merged: Vec<Value>, cloud: &[Value], local: &[Value], op: i8) -> Result<Vec<Value>> {
        let _ = (cloud, local, op);
        Ok(merged)
    }

    async fn export_current(&self) -> Result<Value>;

    async fn import_cloud(&self, data: Value) -> Result<()>;
}

pub async fn export_current<S>(service: &S) -> Result<Value>
where
    S: BaseService + Sync,
    <S::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
{
    let items = service.find_all().await?;
    let list: Vec<<S::Entity as EntityTrait>::Model> = items.into_iter().map(|ptr| ptr.read().clone()).collect();
    Ok(serde_json::to_value(list)?)
}

pub async fn import_cloud<S>(service: &S, data: Value, skip_id: Option<&str>) -> Result<()>
where
    S: BaseService + Sync,
    <S::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<S::Entity as EntityTrait>::ActiveModel> + Send + Sync,
    <S::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = S::Entity> + Send,
    <<S::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<String> + Send,
{
    let items: Vec<<S::Entity as EntityTrait>::Model> = serde_json::from_value(data)?;
    let mut keep_ids = std::collections::HashSet::<String>::new();
    for item in items {
        let Some(id) = item.get_id() else {
            warn!("[sync] import_cloud skip empty id");
            continue;
        };
        if Some(id.as_str()) == skip_id {
            warn!("[sync] import_cloud skip id={}", id);
            continue;
        }
        let write = match service.find_by_id(id.clone()).await? {
            Some(_) => service.update(item).await,
            None => service.insert(item).await,
        };
        match write {
            Ok(_) => {
                keep_ids.insert(id);
            }
            Err(e) => match duplicate_existing_id(&e) {
                Some(existing_id) => {
                    warn!("[sync] import_cloud skip duplicate id={}", id);
                    keep_ids.insert(existing_id.to_string());
                }
                None => return Err(e),
            },
        }
    }
    let existing = service.find_all().await?;
    for ptr in existing {
        let Some(id) = ptr.read().get_id() else {
            warn!("[sync] import_cloud skip empty id");
            continue;
        };
        if Some(id.as_str()) == skip_id {
            warn!("[sync] import_cloud skip id={}", id);
            continue;
        }
        if !keep_ids.contains(&id) {
            service.delete_by_id(id).await?;
        }
    }
    Ok(())
}

fn duplicate_existing_id(err: &anyhow::Error) -> Option<&str> {
    let biz = err.downcast_ref::<BizError>()?;
    if biz.code != ResultCode::Duplicate.as_i32() {
        return None;
    }
    Some(biz.message.as_str())
}

pub fn keep_referenced(merged: Vec<Value>, field: &str, referenced: &HashSet<String>) -> Result<Vec<Value>> {
    let mut order: Vec<String> = Vec::new();
    let mut buckets: HashMap<String, Vec<Value>> = HashMap::new();
    for item in merged {
        let key = object_string(&item, field)?;
        if !buckets.contains_key(&key) {
            order.push(key.clone());
        }
        buckets.entry(key).or_default().push(item);
    }
    let mut result = Vec::new();
    for key in order {
        let items = buckets.remove(&key).context("keep_referenced bucket missing")?;
        let mut kept = Vec::new();
        for item in &items {
            let id = object_string(item, "id")?;
            if referenced.contains(&id) {
                kept.push(item.clone());
            }
        }
        if kept.is_empty() {
            let first = items.into_iter().next().context("keep_referenced empty bucket")?;
            result.push(first);
        } else {
            result.extend(kept);
        }
    }
    Ok(result)
}

fn object_string(item: &Value, key: &str) -> Result<String> {
    let value = item.get(key).with_context(|| format!("cloud sync item missing {}", key))?;
    let Value::String(value) = value else {
        bail!("cloud sync item {} must be string", key);
    };
    if value.is_empty() {
        bail!("cloud sync item {} is empty", key);
    }
    Ok(value.clone())
}

#[macro_export]
macro_rules! submit_sync_entry {
    ($ty:ty) => {
        ::inventory::submit! {
            $crate::service::service_base_sync::SyncEntry {
                fetch: |app| {
                    use ::summer::plugin::ComponentRegistry;
                    app.try_get_component::<$ty>()
                        .ok()
                        .map(|comp| {
                            ::std::sync::Arc::new(comp)
                                as ::std::sync::Arc<dyn $crate::service::service_base_sync::BaseSync>
                        })
                },
            }
        }
    };
}

pub struct SyncEntry {
    pub fetch: fn(&App) -> Option<Arc<dyn BaseSync>>,
}

inventory::collect!(SyncEntry);

static SYNCERS: OnceLock<Vec<Arc<dyn BaseSync>>> = OnceLock::new();

pub fn wire(app: &App) {
    let mut items: Vec<Arc<dyn BaseSync>> = Vec::new();
    for reg in inventory::iter::<SyncEntry> {
        items.push((reg.fetch)(app).expect("sync service missing"));
    }
    items.sort_by_key(|item| item.sync_type());
    for pair in items.windows(2) {
        if pair[0].sync_type() == pair[1].sync_type() {
            panic!("duplicate cloud sync type {}", pair[0].sync_type());
        }
    }
    if SYNCERS.set(items).is_err() {
        panic!("cloud sync already wired");
    }
}

pub fn all() -> &'static [Arc<dyn BaseSync>] {
    SYNCERS.get().expect("cloud sync not wired")
}
