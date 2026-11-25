use summer::plugin::Service;
use termini_macros::base_dao;
use crate::model::command_allow;
use crate::dao::dao_base::BaseDao;
use crate::model::dto::shared_ptr::Shared;
use crate::model::command_allow::CommandAllow;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(command_allow::Entity, sort = 120, phase = 30, cache_all = true)]
#[derive(Clone, Service)]
pub struct CommandAllowDao {}

impl CommandAllowDao {
    pub fn exists(&self, device_id: &str, tool_id: &str, command_hash: &str) -> bool {
        self.find(device_id, tool_id, command_hash).is_some()
    }

    pub fn find(&self, device_id: &str, tool_id: &str, command_hash: &str) -> Option<Shared<CommandAllow>> {
        for entry in self.cache_map().iter() {
            let ptr = entry.value();
            let allow = ptr.read();
            if allow.device_id.as_str() == device_id && allow.tool_id.as_str() == tool_id && allow.command_hash.as_str() == command_hash {
                drop(allow);
                return Some(ptr.clone());
            }
        }
        None
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> anyhow::Result<u64> {
        let conn = self.get_conn()?;
        let result = command_allow::Entity::delete_many().filter(command_allow::Column::DeviceId.eq(device_id.clone())).exec(&conn).await?;
        if self.cache_all() {
            let stale_ids: Vec<String> = self.cache_map().iter().filter(|entry| entry.value().read().device_id.as_str() == device_id.as_str()).map(|entry| entry.key().clone()).collect();
            for id in stale_ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }
}
