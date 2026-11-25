use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::dao::dao_base::BaseDao;
use crate::model::job_indicator_alert;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(job_indicator_alert::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct JobIndicatorAlertDao {}

#[async_trait]
impl BaseApp for JobIndicatorAlertDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}

impl JobIndicatorAlertDao {
    pub async fn delete_by_ids(&self, ids: Vec<String>) -> anyhow::Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let conn = self.get_conn()?;
        let result = job_indicator_alert::Entity::delete_many().filter(job_indicator_alert::Column::Id.is_in(ids.clone())).exec(&conn).await?;
        if self.cache_all() {
            for id in ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }

    pub async fn delete_by_job_device_ids(&self, job_device_ids: Vec<String>) -> anyhow::Result<u64> {
        if job_device_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.get_conn()?;
        let result = job_indicator_alert::Entity::delete_many().filter(job_indicator_alert::Column::JobDeviceId.is_in(job_device_ids.clone())).exec(&conn).await?;
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
