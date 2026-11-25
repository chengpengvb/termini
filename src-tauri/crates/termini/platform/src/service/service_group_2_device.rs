use std::sync::Arc;
use dashmap::DashMap;
use serde_json::Value;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::group_2_device;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use anyhow::{bail, Context, Result};
use crate::dao::dao_group::GroupDao;
use termini_base::submit_sync_entry;
use tokio::sync::Mutex as AsyncMutex;
use std::collections::{HashMap, HashSet};
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::service::service_base::BaseService;
use crate::model::group_2_device::Group2Device;
use crate::dao::dao_group_2_device::Group2DeviceDao;
use crate::model::resp::resp_model::{BizError, ResultCode};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use crate::service::service_base_sync::{self, BaseSync, OP_DOWNLOAD};

fn create_device_locks() -> Arc<DashMap<String, Arc<AsyncMutex<()>>>> {
    Arc::new(DashMap::new())
}

#[base_service(group_2_device::Entity, dao = "group_2_device_dao", service = false)]
#[derive(Clone, Service)]
pub struct Group2DeviceService {
    #[inject(component)]
    group_dao: GroupDao,

    #[inject(component)]
    group_2_device_dao: Group2DeviceDao,

    #[inject(func = create_device_locks())]
    device_locks: Arc<DashMap<String, Arc<AsyncMutex<()>>>>,
}

#[async_trait]
impl BaseService for Group2DeviceService {
    type Entity = group_2_device::Entity;

    fn get_dao(&self) -> Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.group_2_device_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let lock = self.device_locks.entry(entity.device_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = lock.lock().await;
        self.insert_pair(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let lock = self.device_locks.entry(entity.device_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = lock.lock().await;
        self.update_pair(entity).await
    }
}

impl Group2DeviceService {
    async fn insert_pair(&self, entity: Group2Device) -> Result<Shared<Group2Device>> {
        let existing = self.find_by_device_id_all(&entity.device_id).await?;
        if let Some(found) = existing.into_iter().find(|item| item.read().group_id == entity.group_id) {
            return Err(BizError::duplicate(found.read().id.clone()).into());
        }
        self.get_dao()?.insert(entity).await
    }

    async fn update_pair(&self, entity: Group2Device) -> Result<Shared<Group2Device>> {
        let existing = self.find_by_device_id_all(&entity.device_id).await?;
        for found in &existing {
            let row = found.read();
            if row.id != entity.id && row.group_id == entity.group_id {
                return Err(BizError::duplicate(row.id.clone()).into());
            }
        }
        self.get_dao()?.update(entity).await
    }

    async fn find_by_device_id_all(&self, device_id: &str) -> Result<Vec<Shared<Group2Device>>> {
        let list = self.find_all().await?;
        Ok(list.into_iter().filter(|item| item.read().device_id == device_id).collect())
    }

    async fn delete_old_device_2_group(&self, list: Vec<Shared<Group2Device>>) -> Result<()> {
        for item in list {
            let id = item.read().id.clone();
            self.delete_by_id(id).await?;
        }
        Ok(())
    }

    pub async fn find_by_device_id(&self, device_id: &str) -> Option<Shared<Group2Device>> {
        let Ok(list) = self.find_all().await else {
            return None;
        };
        list.into_iter().filter(|item| item.read().device_id == device_id).next()
    }

    pub async fn find_by_device_id_2_group_map(&self) -> DashMap<String, Vec<Group2Device>> {
        let Ok(list) = self.find_all().await else {
            return DashMap::new();
        };
        let map: DashMap<String, Vec<Group2Device>> = DashMap::new();
        for item in list {
            let row = item.read().clone();
            let mut groups = map.entry(row.device_id.clone()).or_default();
            if groups.iter().any(|old| old.group_id == row.group_id) {
                continue;
            }
            groups.push(row);
        }
        map
    }

    pub fn referenced_group_ids(&self) -> HashSet<String> {
        self.group_2_device_dao.cache_map().iter().map(|entry| entry.value().read().group_id.clone()).collect()
    }

    pub async fn set_device_2_group(&self, group_id: &str, device_id: &str) -> Result<()> {
        let lock = self.device_locks.entry(device_id.to_string()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = lock.lock().await;
        match self.group_dao.find_by_id(group_id.to_string()).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(ResultCode::GroupNotFound.into());
            }
            Err(e) => {
                return Err(e);
            }
        }
        let mut keep = None;
        let mut duplicates = Vec::new();
        let list = self.find_by_device_id_all(device_id).await?;
        for item in list {
            if item.read().group_id != group_id {
                continue;
            }
            if keep.is_some() {
                duplicates.push(item);
            } else {
                keep = Some(item);
            }
        }
        self.delete_old_device_2_group(duplicates).await?;
        if keep.is_some() {
            return Ok(());
        }
        let group_2_device = Group2Device { id: uuid(), device_id: device_id.to_string(), group_id: group_id.to_string() };
        self.insert_pair(group_2_device).await?;
        Ok(())
    }

    pub async fn reassign_group(&self, from_group_id: &str, to_group_id: &str) -> Result<()> {
        let list = self.find_all().await?;
        let rows: Vec<Shared<Group2Device>> = list.into_iter().filter(|item| item.read().group_id == from_group_id).collect();
        for item in rows {
            let device_id = item.read().device_id.clone();
            let id = item.read().id.clone();
            self.set_device_2_group(to_group_id, &device_id).await?;
            self.delete_by_id(id).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl BaseSync for Group2DeviceService {
    fn sync_type(&self) -> i8 {
        3
    }

    fn merged(&self, merged: Vec<Value>, cloud: &[Value], local: &[Value], op: i8) -> Result<Vec<Value>> {
        let cloud_pairs = index_group_2_device_pairs(cloud)?;
        let local_pairs = index_group_2_device_pairs(local)?;
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut result = Vec::new();
        for item in merged {
            let key = object_pair(&item)?;
            if !seen.insert(key.clone()) {
                continue;
            }
            if cloud_pairs.contains_key(&key) && local_pairs.contains_key(&key) {
                let winner =
                    if op == OP_DOWNLOAD { cloud_pairs.get(&key).context("cloud group_2_device pair missing")? } else { local_pairs.get(&key).context("local group_2_device pair missing")? };
                result.push((*winner).clone());
            } else {
                result.push(item);
            }
        }
        Ok(result)
    }

    async fn export_current(&self) -> Result<Value> {
        service_base_sync::export_current(self).await
    }

    async fn import_cloud(&self, data: Value) -> Result<()> {
        service_base_sync::import_cloud(self, data, None).await
    }
}

submit_sync_entry!(Group2DeviceService);

fn index_group_2_device_pairs(items: &[Value]) -> Result<HashMap<(String, String), &Value>> {
    let mut map: HashMap<(String, String), &Value> = HashMap::new();
    for item in items {
        let key = object_pair(item)?;
        if map.contains_key(&key) {
            continue;
        }
        map.insert(key, item);
    }
    Ok(map)
}

fn object_pair(item: &Value) -> Result<(String, String)> {
    Ok((object_string(item, "device_id")?, object_string(item, "group_id")?))
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
