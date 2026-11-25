use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::model::account_token;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::{AppContext, BaseApp};

#[base_dao(account_token::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct AccountTokenDao {}

#[async_trait]
impl BaseApp for AccountTokenDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}
