use crate::model::ai_chat;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::dao::dao_base::BaseDao;
use crate::model::ai_chat::{AiChat, AiChatType};
use crate::base::base_app::{AppContext, BaseApp};

#[base_dao(ai_chat::Entity, sort = 120, phase = 30, cache_all = true, custom_start = true)]
#[derive(Clone, Service)]
pub struct AiChatDao {}

#[async_trait]
impl BaseApp for AiChatDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        Ok(())
    }
}

impl AiChatDao {
    pub fn list_by_type(&self, r#type: AiChatType) -> Vec<AiChat> {
        self.cache_map().iter().filter(|entry| entry.value().read().r#type == r#type as i16).map(|entry| entry.value().read().clone()).collect()
    }
}
