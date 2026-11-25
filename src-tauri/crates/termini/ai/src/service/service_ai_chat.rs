use regex::Regex;
use anyhow::Result;
use std::sync::OnceLock;
use sea_orm::EntityTrait;
use crate::model::ai_chat;
use summer::plugin::Service;
use async_trait::async_trait;
use std::collections::HashSet;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::dao::dao_ai_chat::AiChatDao;
use crate::module::ai::shared::WakeBus;
use crate::util::time_util::now_millis;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::web::ws::ws_manager::WsManager;
use crate::module::ai::shared::ChatFileStore;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::model::ai_chat::{AiChat, AiChatType};
use crate::model::req::req_ai_chat::ReqAiChatRevert;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::service::service_ai_chat_memory::AiChatMemoryService;
use crate::service::service_ai_file_change::AiFileChangeService;
use crate::model::resp::event::resp_event_ai_chat::RespEventAiChat;
use crate::service::service_ai_chat_2_project::AiChat2ProjectService;
use crate::service::service_ai_chat_memory_snapshot::AiChatMemorySnapshotService;
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;

const TITLE_MAX_CHARS: usize = 60;
const DEFAULT_CHAT_TITLE: &str = "new chat";

fn regex_title_token() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"@\[[a-z]+:[^]]+]").expect("chat title token regex"))
}

#[derive(Clone, Service)]
pub struct AiChatService {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    ai_chat_dao: AiChatDao,

    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    chat_file_store: ChatFileStore,

    #[inject(component)]
    ai_chat_item_service: AiChatItemService,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,

    #[inject(component)]
    ai_chat_memory_service: AiChatMemoryService,

    #[inject(component)]
    ai_file_change_service: AiFileChangeService,

    #[inject(component)]
    ai_chat_2_project_service: AiChat2ProjectService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(component)]
    ai_chat_memory_snapshot_service: AiChatMemorySnapshotService,
}

#[async_trait]
impl BaseService for AiChatService {
    type Entity = ai_chat::Entity;

    fn get_dao(&self) -> Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.ai_chat_dao.clone()))
    }

    async fn find_all(&self) -> Result<Vec<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
    {
        let mut list = self.ai_chat_dao.find_all().await?;
        list.sort_by(|a, b| b.read().timestamp.cmp(&a.read().timestamp));
        Ok(list)
    }

    async fn insert(&self, _entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + sea_orm::IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: sea_orm::ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let count = self.find_all().await?.len();
        let entity = AiChat { id: uuid(), sort: count as i16, timestamp: now_millis(), title: DEFAULT_CHAT_TITLE.to_string(), r#type: AiChatType::Chat as i16 };
        let saved = self.ai_chat_dao.insert(entity).await?;
        let chat_id = saved.read().id.clone();
        self.emit(RespEventAiChat::OnChanged(chat_id));
        Ok(saved)
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + sea_orm::IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: sea_orm::ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let saved = self.ai_chat_dao.update(entity).await?;
        let chat_id = saved.read().id.clone();
        self.emit(RespEventAiChat::OnChanged(chat_id));
        Ok(saved)
    }

    async fn delete_by_id(&self, id: String) -> Result<u64>
    where
        <<Self::Entity as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<String> + Send,
    {
        if let Some(req_id) = self.chat_session_cache_manager.get_req_id_by_chat_id(&id) {
            self.chat_session_cache_manager.mark_deleting(&id);
            let _ = self.chat_session_cache_manager.cancel(&req_id);
        }
        self.wake_bus.release(&id);
        self.local_shell_registry.release(&id);
        let rows = self.ai_chat_dao.delete_by_id(id.clone()).await?;
        self.ai_chat_item_service.delete_by_chat_id(id.clone()).await?;
        self.ai_chat_memory_service.delete_by_chat_id(id.clone()).await?;
        self.ai_file_change_service.delete_by_chat_id(id.clone()).await?;
        self.ai_chat_2_project_service.delete_by_chat(&id).await?;
        if let Err(error) = self.chat_file_store.remove_chat(&id).await {
            warn!("[ai] 删除对话文件目录失败 chat_id={} error={:#}", id, error);
        }
        self.chat_session_cache_manager.unmark_deleting(&id);
        self.emit(RespEventAiChat::OnDeleted(id));
        Ok(rows)
    }
}

impl AiChatService {
    fn emit(&self, event: RespEventAiChat) {
        self.ws_manager.broadcast_all(&event.to_resp());
    }

    pub async fn touch(&self, id: String) -> Result<()> {
        let Some(shared) = self.find_by_id(id.clone()).await? else {
            return Err(ResultCode::ChatNotFound.into());
        };
        let mut chat = shared.read().clone();
        chat.timestamp = now_millis();
        self.update(chat).await?;
        Ok(())
    }

    pub fn needs_title(&self, chat_id: &str) -> bool {
        self.list_by_type(AiChatType::Chat).iter().any(|chat| chat.id == chat_id && chat.title == DEFAULT_CHAT_TITLE)
    }

    pub async fn rename(&self, id: String, title: String) -> Result<()> {
        if !self.needs_title(&id) {
            return Ok(());
        }
        let Some(shared) = self.find_by_id(id.clone()).await? else {
            return Err(ResultCode::ChatNotFound.into());
        };
        let mut chat = shared.read().clone();
        chat.title = Self::generate_title(&title);
        self.update(chat).await?;
        Ok(())
    }

    pub async fn rename_by_user(&self, id: String, title: String) -> Result<()> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(ResultCode::ChatTitleEmpty.into());
        }
        let Some(shared) = self.find_by_id(id.clone()).await? else {
            return Err(ResultCode::ChatNotFound.into());
        };
        let mut chat = shared.read().clone();
        chat.title = Self::generate_title(&title);
        self.update(chat).await?;
        Ok(())
    }

    fn generate_title(title: &str) -> String {
        let regex = regex_title_token();
        let tokens: Vec<(usize, usize)> = regex.find_iter(title).map(|m| (m.start(), m.end())).collect();
        let mut units: Vec<(usize, usize)> = Vec::new();
        let mut cursor = 0usize;
        let mut match_index = 0usize;
        while cursor < title.len() {
            if match_index < tokens.len() && tokens[match_index].0 == cursor {
                let (start, end) = tokens[match_index];
                units.push((start, end));
                cursor = end;
                match_index += 1;
                continue;
            }
            let next = title[cursor..].chars().next().expect("title cursor must land on a char boundary");
            let step = next.len_utf8();
            units.push((cursor, cursor + step));
            cursor += step;
        }
        if units.len() <= TITLE_MAX_CHARS {
            return title.to_string();
        }
        let (_, end) = units[TITLE_MAX_CHARS - 1];
        format!("{}...", &title[..end])
    }

    pub async fn start_task(&self, r#type: AiChatType, title: String) -> Result<AiChat> {
        let chat = AiChat { id: uuid(), sort: 0, timestamp: now_millis(), title: Self::generate_title(&title), r#type: r#type as i16 };
        let saved = self.ai_chat_dao.insert(chat).await?;
        let chat = saved.read().clone();
        self.emit(RespEventAiChat::OnChanged(chat.id.clone()));
        Ok(chat)
    }

    pub async fn revert_from_item(&self, req: ReqAiChatRevert) -> Result<()> {
        if self.chat_session_cache_manager.get_req_id_by_chat_id(&req.chat_id).is_some() {
            return Err(ResultCode::ChatGenerating.into());
        }
        let items = self.ai_chat_item_service.select_by_chat_id(req.chat_id.clone()).await?;
        let Some(item_index) = items.iter().position(|item| item.read().id == req.item_id) else {
            return Err(ResultCode::ChatItemNotFound.into());
        };
        let removed_ids: HashSet<String> = items.iter().skip(item_index).map(|item| item.read().id.clone()).collect();

        if let Some(snapshot) = self.ai_chat_memory_snapshot_service.load_snapshot(req.chat_id.clone(), req.item_id.clone()).await? {
            let restored = snapshot.turns.len();
            self.ai_chat_memory_service.save_turns(req.chat_id.clone(), snapshot.turns, snapshot.summary, snapshot.scene).await?;
            let ids: Vec<String> = items.into_iter().skip(item_index).map(|item| item.read().id.clone()).collect();
            let rows = self.ai_chat_item_service.delete_by_ids(req.chat_id.clone(), ids).await?;
            info!("[ai] 重新发起回退完成(快照恢复) chat_id={} item_id={} 恢复分组:{} 删除消息:{}", req.chat_id, req.item_id, restored, rows);
            return Ok(());
        }

        let loaded = self.ai_chat_memory_service.load(req.chat_id.clone()).await?;
        let mut turns = loaded.turns;
        let turn_index = turns.iter().position(|turn| removed_ids.contains(&turn.item_id)).unwrap_or(turns.len());
        turns.truncate(turn_index);
        self.ai_chat_memory_service.save_turns(req.chat_id.clone(), turns, loaded.summary, loaded.scene).await?;

        let ids: Vec<String> = items.into_iter().skip(item_index).map(|item| item.read().id.clone()).collect();
        let rows = self.ai_chat_item_service.delete_by_ids(req.chat_id.clone(), ids).await?;
        info!("[ai] 重新发起回退完成(截断兜底) chat_id={} item_id={} 保留分组:{} 删除消息:{}", req.chat_id, req.item_id, turn_index, rows);
        Ok(())
    }

    pub fn list_by_type(&self, r#type: AiChatType) -> Vec<AiChat> {
        let mut list = self.ai_chat_dao.list_by_type(r#type);
        list.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        list
    }
}
