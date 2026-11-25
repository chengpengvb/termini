use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::model::port_forward;
use crate::dao::dao_base::BaseDao;
use crate::model::dto::shared_ptr::Shared;
use crate::model::port_forward::PortForward;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(port_forward::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct PortForwardDao {}

#[async_trait]
impl BaseApp for PortForwardDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}

impl PortForwardDao {
    pub fn get_by_device_id(&self, device_id: String) -> Vec<Shared<PortForward>> {
        let mut list = Vec::new();
        for entry in self.cache_map().iter() {
            let ptr = entry.value();
            if ptr.read().device_id.as_str() == device_id.as_str() {
                list.push(ptr.clone());
            }
        }
        list
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> Result<u64> {
        let conn = self.get_conn()?;
        let result = port_forward::Entity::delete_many().filter(port_forward::Column::DeviceId.eq(device_id.clone())).exec(&conn).await?;
        if self.cache_all() {
            let stale_ids: Vec<String> = self.cache_map().iter().filter(|entry| entry.value().read().device_id.as_str() == device_id.as_str()).map(|entry| entry.key().clone()).collect();
            for id in stale_ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }
}
