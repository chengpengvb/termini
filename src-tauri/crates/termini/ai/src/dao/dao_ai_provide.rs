use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::ai_provide;
use termini_macros::base_dao;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::{AppContext, BaseApp};

#[base_dao(ai_provide::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct AiProvideDao {}

#[async_trait]
impl BaseApp for AiProvideDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}
