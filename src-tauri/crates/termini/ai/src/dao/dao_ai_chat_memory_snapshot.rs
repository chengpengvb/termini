use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_dao;
use sea_orm::sea_query::Index;
use crate::dao::dao_base::BaseDao;
use crate::model::ai_chat_memory_snapshot;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

#[base_dao(ai_chat_memory_snapshot::Entity, sort = 120, phase = 30, cache_all = false, custom_start = true)]
#[derive(Clone, Service)]
pub struct AiChatMemorySnapshotDao {}

#[async_trait]
impl BaseApp for AiChatMemorySnapshotDao {
    fn phase(&self) -> i16 {
        30
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init().await?;
        self.init_chat_id_index().await?;
        Ok(())
    }
}

impl AiChatMemorySnapshotDao {
    async fn init_chat_id_index(&self) -> Result<()> {
        let conn = self.get_conn()?;
        let backend = conn.get_database_backend();
        let stmt = Index::create().if_not_exists().name("idx_ai_chat_memory_snapshot_chat_id").table(ai_chat_memory_snapshot::Entity).col(ai_chat_memory_snapshot::Column::ChatId).to_owned();
        conn.execute(backend.build(&stmt)).await?;
        Ok(())
    }

    pub async fn find_by_item_id(&self, chat_id: String, item_id: String) -> Result<Option<ai_chat_memory_snapshot::Model>> {
        let conn = self.get_conn()?;
        let found = ai_chat_memory_snapshot::Entity::find().filter(ai_chat_memory_snapshot::Column::ChatId.eq(chat_id)).filter(ai_chat_memory_snapshot::Column::ItemId.eq(item_id)).one(&conn).await?;
        Ok(found)
    }

    pub async fn find_by_chat_id_desc(&self, chat_id: String) -> Result<Vec<ai_chat_memory_snapshot::Model>> {
        let conn = self.get_conn()?;
        let list = ai_chat_memory_snapshot::Entity::find().filter(ai_chat_memory_snapshot::Column::ChatId.eq(chat_id)).order_by_desc(ai_chat_memory_snapshot::Column::Timestamp).all(&conn).await?;
        Ok(list)
    }

    pub async fn delete_by_chat_id(&self, chat_id: String) -> Result<u64> {
        let conn = self.get_conn()?;
        let result = ai_chat_memory_snapshot::Entity::delete_many().filter(ai_chat_memory_snapshot::Column::ChatId.eq(chat_id)).exec(&conn).await?;
        Ok(result.rows_affected)
    }

    pub async fn delete_by_ids(&self, ids: Vec<String>) -> Result<u64> {
        let conn = self.get_conn()?;
        let result = ai_chat_memory_snapshot::Entity::delete_many().filter(ai_chat_memory_snapshot::Column::Id.is_in(ids)).exec(&conn).await?;
        Ok(result.rows_affected)
    }
}
