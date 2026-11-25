use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::model::config_key_pair;
use crate::service::service_base::BaseService;
use crate::dao::dao_config_key_pair::ConfigKeyPairDao;

#[base_service(config_key_pair::Entity, dao = "config_key_pair_dao", sync = 7)]
#[derive(Clone, Service)]
pub struct ConfigKeyPairService {
    #[inject(component)]
    config_key_pair_dao: ConfigKeyPairDao,
}

#[async_trait]
impl BaseService for ConfigKeyPairService {
    type Entity = config_key_pair::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.config_key_pair_dao.clone()))
    }
}
