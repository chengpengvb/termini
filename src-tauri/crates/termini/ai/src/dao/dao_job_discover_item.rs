use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::dao::dao_base::BaseDao;
use crate::model::job_discover_item;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::job_discover_item::JobDiscoverItem;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(job_discover_item::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct JobDiscoverItemDao {}

#[async_trait]
impl BaseApp for JobDiscoverItemDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}

impl JobDiscoverItemDao {
    pub fn list_by_discover_id(&self, discover_id: &str) -> Vec<JobDiscoverItem> {
        let mut list: Vec<JobDiscoverItem> = self.cache_map().iter().filter(|entry| entry.value().read().discover_id == discover_id).map(|entry| entry.value().read().clone()).collect();
        list.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
        list
    }

    pub async fn delete_by_discover_ids(&self, discover_ids: Vec<String>) -> anyhow::Result<u64> {
        if discover_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.get_conn()?;
        let result = job_discover_item::Entity::delete_many().filter(job_discover_item::Column::DiscoverId.is_in(discover_ids.clone())).exec(&conn).await?;
        if self.cache_all() {
            let stale_ids: Vec<String> =
                self.cache_map().iter().filter(|entry| discover_ids.iter().any(|id| entry.value().read().discover_id.as_str() == id.as_str())).map(|entry| entry.key().clone()).collect();
            for id in stale_ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }
}
