use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use crate::model::ai_chat_item;
use crate::dao::dao_base::BaseDao;
use sea_orm::sea_query::{Expr, Index};
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

#[base_dao(ai_chat_item::Entity, sort = 120, phase = 30, cache_all = false, custom_start = true)]
#[derive(Clone, Service)]
pub struct AiChatItemDao {}

#[async_trait]
impl BaseApp for AiChatItemDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        self.init_chat_id_index().await?;
        Ok(())
    }
}

impl AiChatItemDao {
    async fn init_chat_id_index(&self) -> Result<()> {
        let conn = self.get_conn()?;
        let backend = conn.get_database_backend();
        let stmt = Index::create().if_not_exists().name("idx_ai_chat_item_chat_id").table(ai_chat_item::Entity).col(ai_chat_item::Column::ChatId).to_owned();
        conn.execute(backend.build(&stmt)).await?;
        Ok(())
    }

    pub async fn delete_by_chat_id(&self, chat_id: String) -> Result<u64> {
        let conn = self.get_conn()?;
        let result = ai_chat_item::Entity::delete_many().filter(ai_chat_item::Column::ChatId.eq(chat_id)).exec(&conn).await?;
        Ok(result.rows_affected)
    }

    pub async fn delete_by_ids(&self, ids: Vec<String>) -> Result<u64> {
        let conn = self.get_conn()?;
        let result = ai_chat_item::Entity::delete_many().filter(ai_chat_item::Column::Id.is_in(ids)).exec(&conn).await?;
        Ok(result.rows_affected)
    }

    pub async fn find_by_chat_id(&self, chat_id: String) -> Result<Vec<ai_chat_item::Model>> {
        let conn = self.get_conn()?;
        let list = ai_chat_item::Entity::find().filter(ai_chat_item::Column::ChatId.eq(chat_id)).all(&conn).await?;
        Ok(list)
    }

    pub async fn find_chat_ids_by_keyword(&self, keyword: &str, types: &[i16]) -> Result<Vec<String>> {
        let conn = self.get_conn()?;
        let escaped = keyword.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{}%", escaped);
        let ids: Vec<String> = ai_chat_item::Entity::find()
            .select_only()
            .column(ai_chat_item::Column::ChatId)
            .distinct()
            .filter(ai_chat_item::Column::Type.is_in(types.iter().copied()))
            .filter(Expr::col(ai_chat_item::Column::Context).like(pattern))
            .into_tuple()
            .all(&conn)
            .await?;
        Ok(ids)
    }
}
