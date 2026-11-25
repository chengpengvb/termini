use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use sea_orm::sea_query::Index;
use crate::model::ai_file_change;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

#[base_dao(ai_file_change::Entity, sort = 121, phase = 30, cache_all = false, custom_start = true)]
#[derive(Clone, Service)]
pub struct AiFileChangeDao {}

#[async_trait]
impl BaseApp for AiFileChangeDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        self.init_chat_id_index().await?;
        Ok(())
    }
}

impl AiFileChangeDao {
    async fn init_chat_id_index(&self) -> Result<()> {
        let conn = self.get_conn()?;
        let backend = conn.get_database_backend();
        let stmt = Index::create().if_not_exists().name("idx_ai_file_change_chat_id").table(ai_file_change::Entity).col(ai_file_change::Column::ChatId).to_owned();
        conn.execute(backend.build(&stmt)).await?;
        Ok(())
    }

    pub async fn find_by_chat_id(&self, chat_id: &str) -> Result<Vec<ai_file_change::Model>> {
        let conn = self.get_conn()?;
        let list = ai_file_change::Entity::find().filter(ai_file_change::Column::ChatId.eq(chat_id)).order_by_asc(ai_file_change::Column::Timestamp).all(&conn).await?;
        Ok(list)
    }

    pub async fn delete_by_chat_id(&self, chat_id: String) -> Result<u64> {
        let conn = self.get_conn()?;
        let result = ai_file_change::Entity::delete_many().filter(ai_file_change::Column::ChatId.eq(chat_id)).exec(&conn).await?;
        Ok(result.rows_affected)
    }
}
