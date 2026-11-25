use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::port_forward;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::service::service_base::BaseService;
use crate::dao::dao_port_forward::PortForwardDao;
use crate::model::port_forward::{PortForward, FORWARD_STATUS_IDLE};

fn create_status_map() -> Arc<DashMap<String, i16>> {
    Arc::new(DashMap::new())
}

#[base_service(port_forward::Entity, dao = "port_forward_dao", sync = 8)]
#[derive(Clone, Service)]
pub struct PortForwardService {
    #[inject(component)]
    port_forward_dao: PortForwardDao,

    #[inject(func = create_status_map())]
    status_map: Arc<DashMap<String, i16>>,
}

#[async_trait]
impl BaseService for PortForwardService {
    type Entity = port_forward::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.port_forward_dao.clone()))
    }
}

impl PortForwardService {
    pub async fn select_all_with_status(&self) -> anyhow::Result<Vec<PortForward>> {
        Ok(self.find_all().await?.iter().map(|ptr| self.fill_status(ptr.read().clone())).collect())
    }

    pub fn select_by_device(&self, device_id: String) -> Vec<PortForward> {
        self.port_forward_dao.get_by_device_id(device_id).iter().map(|ptr| self.fill_status(ptr.read().clone())).collect()
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> anyhow::Result<u64> {
        self.port_forward_dao.delete_by_device_id(device_id).await
    }

    pub fn set_status(&self, forward_id: String, status: i16) {
        self.status_map.insert(forward_id, status);
    }

    pub fn get_status(&self, forward_id: &str) -> i16 {
        self.status_map.get(forward_id).map(|entry| *entry.value()).unwrap_or(FORWARD_STATUS_IDLE)
    }

    fn fill_status(&self, mut rule: PortForward) -> PortForward {
        rule.status = self.get_status(&rule.id);
        rule
    }
}
