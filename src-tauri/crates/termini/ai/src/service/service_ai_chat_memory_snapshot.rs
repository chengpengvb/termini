use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use crate::util::time_util::now_millis;
use crate::model::ai_chat_memory_snapshot;
use crate::service::service_base::BaseService;
use crate::model::ai_chat_memory_snapshot::AiChatMemorySnapshot;
use crate::model::dto::chat_memory::{LoadedMemory, MemoryPayload};
use crate::dao::dao_ai_chat_memory_snapshot::AiChatMemorySnapshotDao;

const SNAPSHOT_KEEP_PER_CHAT: usize = 5;

#[derive(Clone, Service)]
pub struct AiChatMemorySnapshotService {
    #[inject(component)]
    ai_chat_memory_snapshot_dao: AiChatMemorySnapshotDao,
}

#[async_trait]
impl BaseService for AiChatMemorySnapshotService {
    type Entity = ai_chat_memory_snapshot::Entity;

    fn get_dao(&self) -> Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.ai_chat_memory_snapshot_dao.clone()))
    }
}

impl AiChatMemorySnapshotService {
    pub async fn save_snapshot(&self, chat_id: String, item_id: String, loaded: &LoadedMemory) -> Result<()> {
        let payload = serde_json::to_value(&MemoryPayload { turns: loaded.turns.clone(), summary: loaded.summary.clone(), scene: loaded.scene.clone() })?;
        let snapshot = AiChatMemorySnapshot { id: uuid(), chat_id: chat_id.clone(), item_id, messages: payload, timestamp: now_millis() };
        self.insert(snapshot).await?;
        let list = self.ai_chat_memory_snapshot_dao.find_by_chat_id_desc(chat_id).await?;
        if list.len() <= SNAPSHOT_KEEP_PER_CHAT {
            return Ok(());
        }
        let stale: Vec<String> = list.into_iter().skip(SNAPSHOT_KEEP_PER_CHAT).map(|row| row.id).collect();
        self.ai_chat_memory_snapshot_dao.delete_by_ids(stale).await?;
        Ok(())
    }

    pub async fn load_snapshot(&self, chat_id: String, item_id: String) -> Result<Option<LoadedMemory>> {
        let Some(snapshot) = self.ai_chat_memory_snapshot_dao.find_by_item_id(chat_id, item_id).await? else {
            return Ok(None);
        };
        let payload = serde_json::from_value::<MemoryPayload>(snapshot.messages)?;
        Ok(Some(LoadedMemory { turns: payload.turns, summary: payload.summary, scene: payload.scene }))
    }

    pub async fn delete_by_chat_id(&self, chat_id: String) -> Result<u64> {
        self.ai_chat_memory_snapshot_dao.delete_by_chat_id(chat_id).await
    }
}
