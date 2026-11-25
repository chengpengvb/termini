use summer::plugin::Service;
use async_trait::async_trait;
use std::collections::HashSet;
use termini_macros::base_service;
use crate::collect::dto::JobModel;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use crate::model::job_device_2_indicator;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::model::resp::resp_model::BizError;
use crate::service::service_base::BaseService;
use crate::model::job_device_2_indicator::JobDevice2Indicator;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use crate::dao::dao_job_device_2_indicator::JobDevice2IndicatorDao;

#[base_service(job_device_2_indicator::Entity, dao = "job_device_2_indicator_dao", sync = 11)]
#[derive(Clone, Service)]
pub struct JobDevice2IndicatorService {
    #[inject(component)]
    job_device_2_indicator_dao: JobDevice2IndicatorDao,
}

#[async_trait]
impl BaseService for JobDevice2IndicatorService {
    type Entity = job_device_2_indicator::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.job_device_2_indicator_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let list = self.find_all().await?;
        if let Some(found) = list.into_iter().find(|item| {
            let row = item.read();
            row.job_device_id == entity.job_device_id && row.job_indicator_id == entity.job_indicator_id
        }) {
            return Err(BizError::duplicate(found.read().id.clone()).into());
        }
        self.get_dao()?.insert(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let list = self.find_all().await?;
        for found in &list {
            let row = found.read();
            if row.id != entity.id && row.job_device_id == entity.job_device_id && row.job_indicator_id == entity.job_indicator_id {
                return Err(BizError::duplicate(row.id.clone()).into());
            }
        }
        self.get_dao()?.update(entity).await
    }
}

impl JobDevice2IndicatorService {
    pub async fn save_link(&self, job_device_id: &str, job_indicator_id: &str, interval_secs: i64) -> anyhow::Result<JobDevice2Indicator> {
        let period = JobModel::clamp_period(interval_secs) as i64;
        for shared in self.find_all().await? {
            let mut link = shared.read().clone();
            if link.job_device_id != job_device_id || link.job_indicator_id != job_indicator_id {
                continue;
            }
            if link.interval_secs == period {
                return Ok(link);
            }
            link.interval_secs = period;
            let saved = self.update(link).await?;
            let saved = saved.read().clone();
            return Ok(saved);
        }
        let link = JobDevice2Indicator { id: uuid(), interval_secs: period, job_device_id: job_device_id.to_string(), job_indicator_id: job_indicator_id.to_string(), job_indicator: None };
        let saved = self.insert(link).await?;
        let saved = saved.read().clone();
        Ok(saved)
    }

    pub fn list_indicator_ids_by_job_device_ids(&self, job_device_ids: &[String]) -> Vec<String> {
        self.job_device_2_indicator_dao.list_indicator_ids_by_job_device_ids(job_device_ids)
    }

    pub fn exists_by_job_indicator_id(&self, job_indicator_id: &str) -> bool {
        self.job_device_2_indicator_dao.exists_by_job_indicator_id(job_indicator_id)
    }

    pub fn referenced_job_device_ids(&self) -> HashSet<String> {
        self.job_device_2_indicator_dao.cache_map().iter().map(|entry| entry.value().read().job_device_id.clone()).collect()
    }

    pub async fn delete_by_job_device_ids(&self, job_device_ids: Vec<String>) -> anyhow::Result<u64> {
        self.job_device_2_indicator_dao.delete_by_job_device_ids(job_device_ids).await
    }
}
