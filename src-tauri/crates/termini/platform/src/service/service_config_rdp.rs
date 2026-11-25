use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::config_rdp;
use std::collections::HashMap;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::model::config_rdp::ConfigRdp;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::dao::dao_config_rdp::ConfigRdpDao;
use crate::model::resp::resp_model::BizError;
use crate::service::service_base::BaseService;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};

#[base_service(config_rdp::Entity, dao = "config_rdp_dao", sync = 14)]
#[derive(Clone, Service)]
pub struct ConfigRdpService {
    #[inject(component)]
    config_rdp_dao: ConfigRdpDao,
}

#[async_trait]
impl BaseService for ConfigRdpService {
    type Entity = config_rdp::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.config_rdp_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        if let Some(found) = self.get_by_device_id(entity.device_id.clone()) {
            return Err(BizError::duplicate(found.read().id.clone()).into());
        }
        self.get_dao()?.insert(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        if let Some(found) = self.get_by_device_id(entity.device_id.clone()) {
            if found.read().id != entity.id {
                return Err(BizError::duplicate(found.read().id.clone()).into());
            }
        }
        self.get_dao()?.update(entity).await
    }
}

impl ConfigRdpService {
    pub fn get_by_device_id(&self, device_id: String) -> Option<Shared<ConfigRdp>> {
        self.config_rdp_dao.get_by_device_id(device_id)
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> anyhow::Result<u64> {
        self.config_rdp_dao.delete_by_device_id(device_id).await
    }

    pub fn get_device_id_map(&self) -> HashMap<String, Shared<ConfigRdp>> {
        self.config_rdp_dao.get_device_id_map()
    }
}
