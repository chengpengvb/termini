use std::time::SystemTime;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::known_host;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::model::known_host::KnownHost;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::dao::dao_known_host::KnownHostDao;
use crate::model::resp::resp_model::BizError;
use crate::service::service_base::BaseService;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};

#[base_service(known_host::Entity, dao = "known_host_dao", service = false)]
#[derive(Clone, Service)]
pub struct KnownHostService {
    #[inject(component)]
    known_host_dao: KnownHostDao,
}

#[async_trait]
impl BaseService for KnownHostService {
    type Entity = known_host::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.known_host_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        if let Some(found) = self.find_by_host_port(entity.host.as_str(), entity.port) {
            return Err(BizError::duplicate(found.read().id.clone()).into());
        }
        self.get_dao()?.insert(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        if let Some(found) = self.find_by_host_port(entity.host.as_str(), entity.port) {
            if found.read().id != entity.id {
                return Err(BizError::duplicate(found.read().id.clone()).into());
            }
        }
        self.get_dao()?.update(entity).await
    }
}

impl KnownHostService {
    pub fn find_by_host_port(&self, host: &str, port: u16) -> Option<Shared<KnownHost>> {
        for entry in self.known_host_dao.cache_map().iter() {
            let ptr = entry.value();
            let snapshot = ptr.read();
            if snapshot.host.as_str() == host && snapshot.port == port {
                return Some(ptr.clone());
            }
        }
        None
    }

    pub async fn upsert_trusted(&self, host: String, port: u16, algorithm: String, sha256: String) -> anyhow::Result<()> {
        let trusted_at = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as i64;
        let existing = self.find_by_host_port(&host, port);
        match existing {
            Some(ptr) => {
                let mut snapshot = ptr.read().clone();
                snapshot.host = host;
                snapshot.port = port;
                snapshot.algorithm = algorithm;
                snapshot.sha256 = sha256;
                snapshot.trusted_at = trusted_at;
                let _ = self.update(snapshot).await?;
                Ok(())
            }
            None => {
                let mut item = KnownHost::default();
                item.host = host;
                item.port = port;
                item.algorithm = algorithm;
                item.sha256 = sha256;
                item.trusted_at = trusted_at;
                let _ = self.insert(item).await?;
                Ok(())
            }
        }
    }
}
