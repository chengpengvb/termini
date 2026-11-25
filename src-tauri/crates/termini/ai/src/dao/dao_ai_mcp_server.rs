use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::model::ai_mcp_server;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::{AppContext, BaseApp};

#[base_dao(ai_mcp_server::Entity, sort = 121, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct AiMcpServerDao {}

#[async_trait]
impl BaseApp for AiMcpServerDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}
