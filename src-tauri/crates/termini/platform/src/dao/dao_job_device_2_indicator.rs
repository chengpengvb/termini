use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::dao::dao_base::BaseDao;
use crate::model::job_device_2_indicator;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(job_device_2_indicator::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct JobDevice2IndicatorDao {}

#[async_trait]
impl BaseApp for JobDevice2IndicatorDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}

impl JobDevice2IndicatorDao {
    pub fn list_indicator_ids_by_job_device_ids(&self, job_device_ids: &[String]) -> Vec<String> {
        if job_device_ids.is_empty() {
            return Vec::new();
        }
        self.cache_map()
            .iter()
            .filter(|entry| job_device_ids.iter().any(|id| entry.value().read().job_device_id.as_str() == id.as_str()))
            .map(|entry| entry.value().read().job_indicator_id.clone())
            .collect()
    }

    pub fn exists_by_job_indicator_id(&self, job_indicator_id: &str) -> bool {
        self.cache_map().iter().any(|entry| entry.value().read().job_indicator_id.as_str() == job_indicator_id)
    }

    pub async fn delete_by_job_device_ids(&self, job_device_ids: Vec<String>) -> anyhow::Result<u64> {
        if job_device_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.get_conn()?;
        let result = job_device_2_indicator::Entity::delete_many().filter(job_device_2_indicator::Column::JobDeviceId.is_in(job_device_ids.clone())).exec(&conn).await?;
        if self.cache_all() {
            let stale_ids: Vec<String> =
                self.cache_map().iter().filter(|entry| job_device_ids.iter().any(|id| entry.value().read().job_device_id.as_str() == id.as_str())).map(|entry| entry.key().clone()).collect();
            for id in stale_ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }
}
