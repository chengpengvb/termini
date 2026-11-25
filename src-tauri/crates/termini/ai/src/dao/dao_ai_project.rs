use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::ai_project;
use termini_macros::base_dao;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::{AppContext, BaseApp};

#[base_dao(ai_project::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct AiProjectDao {}

#[async_trait]
impl BaseApp for AiProjectDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}
