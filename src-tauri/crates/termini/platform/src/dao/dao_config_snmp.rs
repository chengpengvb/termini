use summer::plugin::Service;
use termini_macros::base_dao;
use crate::model::config_snmp;
use std::collections::HashMap;
use crate::dao::dao_base::BaseDao;
use crate::model::config_snmp::ConfigSnmp;
use crate::model::dto::shared_ptr::Shared;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(config_snmp::Entity, sort = 120, phase = 30, cache_all = true)]
#[derive(Clone, Service)]
pub struct ConfigSnmpDao {}

impl ConfigSnmpDao {
    pub fn get_by_device_id(&self, device_id: String) -> Option<Shared<ConfigSnmp>> {
        for entry in self.cache_map().iter() {
            let ptr = entry.value();
            if ptr.read().device_id.as_str() == device_id.as_str() {
                return Some(ptr.clone());
            }
        }
        None
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> anyhow::Result<u64> {
        let conn = self.get_conn()?;
        let result = config_snmp::Entity::delete_many().filter(config_snmp::Column::DeviceId.eq(device_id.clone())).exec(&conn).await?;
        if self.cache_all() {
            let stale_ids: Vec<String> = self.cache_map().iter().filter(|entry| entry.value().read().device_id.as_str() == device_id.as_str()).map(|entry| entry.key().clone()).collect();
            for id in stale_ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }

    pub fn get_device_id_map(&self) -> HashMap<String, Shared<ConfigSnmp>> {
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
