use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::model::job_discover;
use crate::dao::dao_base::BaseDao;
use crate::model::job_discover::JobDiscover;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[base_dao(job_discover::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct JobDiscoverDao {}

#[async_trait]
impl BaseApp for JobDiscoverDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}

impl JobDiscoverDao {
    pub fn find_by_chat_id(&self, chat_id: &str) -> Option<JobDiscover> {
        self.cache_map().iter().find(|entry| entry.value().read().chat_id == chat_id).map(|entry| entry.value().read().clone())
    }

    pub async fn delete_by_ids(&self, ids: Vec<String>) -> anyhow::Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let conn = self.get_conn()?;
        let result = job_discover::Entity::delete_many().filter(job_discover::Column::Id.is_in(ids.clone())).exec(&conn).await?;
        if self.cache_all() {
            for id in ids {
                self.cache_map().remove(&id);
            }
        }
        Ok(result.rows_affected)
    }
}
