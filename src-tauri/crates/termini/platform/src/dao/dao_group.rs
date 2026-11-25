use crate::model::group;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::{AppContext, BaseApp};

#[base_dao(group::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct GroupDao {}

#[async_trait]
impl BaseApp for GroupDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}
