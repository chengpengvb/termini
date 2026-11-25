use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::config_serial;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::model::resp::resp_model::BizError;
use crate::model::config_serial::ConfigSerial;
use crate::service::service_base::BaseService;
use crate::dao::dao_config_serial::ConfigSerialDao;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};

#[base_service(config_serial::Entity, dao = "config_serial_dao", service = false)]
#[derive(Clone, Service)]
pub struct ConfigSerialService {
    #[inject(component)]
    config_serial_dao: ConfigSerialDao,
}

#[async_trait]
impl BaseService for ConfigSerialService {
    type Entity = config_serial::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.config_serial_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        if let Some(found) = self.get_by_port_name(entity.port_name.as_str()) {
            return Err(BizError::duplicate(found.read().id.clone()).into());
        }
        self.get_dao()?.insert(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        if let Some(found) = self.get_by_port_name(entity.port_name.as_str()) {
            if found.read().id != entity.id {
                return Err(BizError::duplicate(found.read().id.clone()).into());
            }
        }
        self.get_dao()?.update(entity).await
    }
}

impl ConfigSerialService {
    pub fn get_by_port_name(&self, port_name: &str) -> Option<Shared<ConfigSerial>> {
        self.config_serial_dao.get_by_port_name(port_name)
    }

    pub async fn save_by_port(&self, mut config: ConfigSerial) -> anyhow::Result<Shared<ConfigSerial>> {
        let Some(existing) = self.get_by_port_name(config.port_name.as_str()) else {
            return self.insert(config).await;
        };
        config.id = existing.read().id.clone();
        self.update(config).await
    }
}
