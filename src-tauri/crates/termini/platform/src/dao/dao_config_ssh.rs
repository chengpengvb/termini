use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::config_ssh;
use termini_macros::base_dao;
use std::collections::HashMap;
use crate::dao::dao_base::BaseDao;
use crate::model::config_ssh::ConfigSsh;
use crate::model::dto::shared_ptr::Shared;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(config_ssh::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct ConfigSshDao {}

#[async_trait]
impl BaseApp for ConfigSshDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}

impl ConfigSshDao {
    pub fn get_by_device_id(&self, device_id: String) -> Option<Shared<ConfigSsh>> {
        for entry in self.cache_map().iter() {
            let ptr = entry.value();
            if ptr.read().device_id.as_str() == device_id.as_str() {
                return Some(ptr.clone());
            }
        }
        None
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> Result<u64> {
        let conn = self.get_conn()?;
        let result = config_ssh::Entity::delete_many().filter(config_ssh::Column::DeviceId.eq(device_id.clone())).exec(&conn).await?;
        if self.cache_all() {
            let stale_ids: Vec<String> = self.cache_map().iter().filter(|entry| entry.value().read().device_id.as_str() == device_id.as_str()).map(|entry| entry.key().clone()).collect();
            for id in stale_ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }

    pub fn get_device_id_map(&self) -> HashMap<String, Shared<ConfigSsh>> {
        let mut config_map = HashMap::new();
        for entry in self.cache_map().iter() {
            let ptr = entry.value();
            let device_id = ptr.read().device_id.clone();
            if device_id.is_empty() {
                continue;
            }
            config_map.insert(device_id, ptr.clone());
        }
        config_map
    }
}
