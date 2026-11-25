use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use rig::completion::Message;
use rig::message::UserContent;
use crate::model::ai_chat_memory;
use crate::dao::dao_base::BaseDao;
use crate::util::time_util::now_millis;
use crate::service::service_base::BaseService;
use crate::model::ai_chat_memory::AiChatMemory;
use crate::util::token_util::estimate_text_tokens;
use crate::dao::dao_ai_chat_memory::AiChatMemoryDao;
use crate::service::service_ai_chat_memory_snapshot::AiChatMemorySnapshotService;
use crate::model::dto::chat_memory::{ChatScene, LoadedMemory, MemoryPayload, MemoryPayloadFlat, MemoryTurn};

const IMAGE_PART_TOKENS: usize = 1600;

#[derive(Clone, Service)]
pub struct AiChatMemoryService {
    #[inject(component)]
    ai_chat_memory_dao: AiChatMemoryDao,

    #[inject(component)]
    ai_chat_memory_snapshot_service: AiChatMemorySnapshotService,
}

#[async_trait]
impl BaseService for AiChatMemoryService {
    type Entity = ai_chat_memory::Entity;

    fn get_dao(&self) -> Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.ai_chat_memory_dao.clone()))
    }
}

impl AiChatMemoryService {
    pub async fn load(&self, chat_id: String) -> Result<LoadedMemory> {
        let Some(shared) = self.find_by_id(chat_id).await? else {
            return Ok(LoadedMemory { turns: Vec::new(), summary: String::new(), scene: None });
        };
        let value = shared.read().messages.clone();
        if let Ok(payload) = serde_json::from_value::<MemoryPayload>(value.clone()) {
            return Ok(LoadedMemory { turns: payload.turns, summary: payload.summary, scene: payload.scene });
        }
        if let Ok(payload) = serde_json::from_value::<MemoryPayloadFlat>(value.clone()) {
            return Ok(LoadedMemory { turns: Self::flat_turns(payload.messages), summary: payload.summary, scene: None });
        }
        let messages: Vec<Message> = serde_json::from_value(value)?;
        Ok(LoadedMemory { turns: Self::flat_turns(messages), summary: String::new(), scene: None })
    }

    fn flat_turns(messages: Vec<Message>) -> Vec<MemoryTurn> {
        if messages.is_empty() {
            return Vec::new();
        }
        vec![MemoryTurn { item_id: String::new(), messages }]
    }

    pub async fn save_turns(&self, chat_id: String, turns: Vec<MemoryTurn>, summary: String, scene: Option<ChatScene>) -> Result<()> {
        let timestamp = now_millis();
        let payload = serde_json::to_value(&MemoryPayload { turns, summary, scene })?;
        if let Some(shared) = self.find_by_id(chat_id.clone()).await? {
            let mut model = shared.read().clone();
            model.messages = payload;
            model.timestamp = timestamp;
            self.update(model).await?;
            return Ok(());
        }
        self.insert(AiChatMemory { id: chat_id, messages: payload, timestamp }).await?;
        Ok(())
    }

    pub async fn delete_by_chat_id(&self, chat_id: String) -> Result<u64> {
        self.ai_chat_memory_snapshot_service.delete_by_chat_id(chat_id.clone()).await?;
        self.delete_by_id(chat_id).await
    }

    pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
        let mut tokens = 0usize;
        for message in messages {
            tokens += Self::estimate_tokens(message);
        }
        tokens
    }

    fn estimate_tokens(message: &Message) -> usize {
        let Message::User { content } = message else {
            return Self::estimate_json_tokens(message);
        };
        if !content.iter().any(|part| matches!(part, UserContent::Image(_))) {
            return Self::estimate_json_tokens(message);
        }
        let mut tokens = 0usize;
        for part in content.iter() {
            match part {
                UserContent::Image(_) => tokens += IMAGE_PART_TOKENS,
                other => tokens += Self::estimate_json_tokens(other),
            }
        }
        tokens.max(1)
    }

    fn estimate_json_tokens<T: serde::Serialize>(value: &T) -> usize {
        let Ok(text) = serde_json::to_string(value) else {
            return 0;
        };
        estimate_text_tokens(&text).max(1)
    }
}
