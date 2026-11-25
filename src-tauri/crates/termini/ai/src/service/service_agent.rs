use base64::Engine;
use summer::plugin::Service;
use async_trait::async_trait;
use rig::completion::Message;
use termini_macros::base_app;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use summer::tracing::log::warn;
use summer::tracing::{error, info};
use crate::model::ai_skill::AiSkill;
use crate::model::dto::shared_ptr::Shared;
use crate::web::ws::ws_manager::WsManager;
use crate::model::ai_chat_item::AiChatItem;
use crate::module::ai::shared::ChatFileStore;
use base64::engine::general_purpose::STANDARD;
use crate::service::service_base::BaseService;
use crate::model::dto::chat_memory::MemoryTurn;
use crate::model::resp::resp_model::ResultCode;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::resp::resp_agent::RespAgentImageData;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::module::ai::flow::flow_agent_skill::FlowAgentSkill;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::service::service_ai_chat_memory::AiChatMemoryService;
use crate::model::req::req_agent_image::{ReqAgentImageRead, ReqAgentImageSave};
use crate::module::ai::shared::mem::synthetic::{strip_images, strip_synthetic};
use crate::model::req::req_generate_skill::{ReqCancelGenerateSkill, ReqGenerateSkill};

#[base_app(sort = 901)]
#[derive(Clone, Service)]
pub struct AgentService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    flow_agent_skill: FlowAgentSkill,

    #[inject(component)]
    ai_chat_item_service: AiChatItemService,

    #[inject(component)]
    ai_chat_memory_service: AiChatMemoryService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for AgentService {
    fn name(&self) -> &'static str {
        "AgentService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl AgentService {
    pub async fn generate_skill(&self, req: ReqGenerateSkill) -> anyhow::Result<Shared<AiSkill>> {
        let cancel = self.flow_agent_skill.begin_task(&req.task_id)?;
        let result = self.flow_agent_skill.extract(req.chat_id, req.ai_provide_id, req.key_points, req.name, req.description, req.content, cancel).await;
        self.flow_agent_skill.end_task(&req.task_id);
        let draft = result?;
        Ok(Shared::new(draft))
    }

    pub fn cancel_generate_skill(&self, req: ReqCancelGenerateSkill) -> anyhow::Result<()> {
        self.flow_agent_skill.cancel_task(&req.task_id)
    }

    pub async fn read_image(&self, req: ReqAgentImageRead) -> anyhow::Result<RespAgentImageData> {
        let (path, media_type) = Self::resolve_chat_image(&req.path).await?;
        let bytes = tokio::fs::read(&path).await.map_err(|_| anyhow::Error::from(ResultCode::ChatImageNotFound))?;
        Ok(RespAgentImageData { data: STANDARD.encode(bytes), media_type: media_type.to_string() })
    }

    pub async fn save_image(&self, req: ReqAgentImageSave) -> anyhow::Result<()> {
        let (path, _) = Self::resolve_chat_image(&req.path).await?;
        let target = PathBuf::from(req.target.trim());
        if target.as_os_str().is_empty() {
            return Err(ResultCode::ChatImageSaveFailed.into());
        }
        tokio::fs::copy(&path, &target).await.map_err(|_| anyhow::Error::from(ResultCode::ChatImageSaveFailed))?;
        Ok(())
    }

    async fn resolve_chat_image(raw: &str) -> anyhow::Result<(PathBuf, &'static str)> {
        let root = tokio::fs::canonicalize(ChatFileStore::root()?).await.map_err(|_| anyhow::Error::from(ResultCode::ChatImageNotFound))?;
        let path = tokio::fs::canonicalize(PathBuf::from(raw)).await.map_err(|_| anyhow::Error::from(ResultCode::ChatImageNotFound))?;
        if !path.starts_with(&root) || !path.is_file() {
            return Err(ResultCode::ChatImageNotFound.into());
        }
        let media_type = Self::image_media_type(&path).ok_or_else(|| anyhow::Error::from(ResultCode::ChatImageNotFound))?;
        Ok((path, media_type))
    }

    fn image_media_type(path: &Path) -> Option<&'static str> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "webp" => Some("image/webp"),
            "gif" => Some("image/gif"),
            _ => None,
        }
    }

    pub async fn save_run_memory(&self, req_id: String) {
        let Some(entry) = self.chat_session_cache_manager.get(&req_id) else {
            warn!("[ai] 未知 req_id: {} ,错误1.", req_id);
            return;
        };
        let turns = entry.turns.lock().clone();
        let scene = entry.scene.lock().clone();
        let summary = entry.summary.lock().clone();
        let messages = strip_images(strip_synthetic(entry.messages_without_addon()));
        let user_item_id = entry.user_item_id.clone();
        let chat_id = entry.context.ai_chat_id.clone();

        drop(entry);
        if self.chat_session_cache_manager.is_deleting(&chat_id) {
            return;
        }

        let size = messages.len();
        let summary_size = summary.chars().count();
        let turns = Self::split_run_turn(&chat_id, turns, messages, user_item_id);
        let turns_size = turns.len();
        if let Err(error) = self.ai_chat_memory_service.save_turns(chat_id.clone(), turns, summary, scene).await {
            warn!("[ai] 保存ai_chat_memory 失败: chat_id=[{}] error={}", chat_id, error);
        } else {
            info!("[ai] 保存ai_chat_memory 成功: chat_id=[{}] 分组:{} 数量:{} 摘要:{}.", chat_id, turns_size, size, summary_size);
        }

        let Some(items) = self.ai_chat_item_service.get_item_cache(chat_id.clone()) else {
            return;
        };
        let transient_ids: HashSet<String> = items.iter().filter(|item| !RespEventAgent::should_persist(item.r#type)).map(|item| item.id.clone()).collect();
        if !transient_ids.is_empty() {
            self.ai_chat_item_service.remove_ids_from_cache(chat_id, &transient_ids);
        }
    }

    fn split_run_turn(chat_id: &str, turns: Vec<MemoryTurn>, messages: Vec<Message>, user_item_id: String) -> Vec<MemoryTurn> {
        let mut rest = messages;
        let mut result = Vec::with_capacity(turns.len() + 1);
        for turn in turns {
            let take = turn.messages.len().min(rest.len());
            if take < turn.messages.len() {
                error!("[ai] 记忆分组与记忆条数对不上: chat_id=[{}] item_id=[{}] 分组:{} 实际:{}", chat_id, turn.item_id, turn.messages.len(), take);
            }
            let messages: Vec<Message> = rest.drain(..take).collect();
            result.push(MemoryTurn { item_id: turn.item_id, messages });
        }
        result.push(MemoryTurn { item_id: user_item_id, messages: rest });
        result
    }

    pub fn handler_agent_event(&self, mut event: RespEventAgent, chat_id: String, id: Option<String>, pid: Option<String>, timestamp: Option<i64>) {
        if !self.should_emit_agent_event(&event) {
            return;
        }
        self.stamp_parent_req_id(&mut event);
        let mut ai_chat_item = event.to_ai_chat_item(chat_id, id, pid);
        if let Some(timestamp) = timestamp {
            ai_chat_item.timestamp = timestamp;
        }
        let resp = RespSocketContext { r#type: ai_chat_item.r#type, session_id: ai_chat_item.chat_id.clone(), payload: serde_json::to_value(&ai_chat_item).ok() };
        self.ws_manager.broadcast(&ai_chat_item.chat_id, &resp);
        self.ai_chat_item_service.push_item_2_cache(ai_chat_item.chat_id.clone(), ai_chat_item.clone());
        self.persist_item(ai_chat_item);
    }

    fn should_emit_agent_event(&self, event: &RespEventAgent) -> bool {
        let Some(req_id) = event.req_id() else {
            return true;
        };
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return false;
        };
        if !entry.cancel_token.is_cancelled() {
            return true;
        }
        matches!(event, RespEventAgent::OnCancelled(_) | RespEventAgent::OnDone(_))
    }

    fn stamp_parent_req_id(&self, event: &mut RespEventAgent) {
        let Some(req_id) = event.req_id().map(str::to_string) else {
            return;
        };
        let Some(parent_req_id) = self.chat_session_cache_manager.parent_req_id(&req_id) else {
            return;
        };
        event.set_parent_req_id(parent_req_id);
    }

    fn persist_item(&self, item: AiChatItem) {
        if !RespEventAgent::should_persist(item.r#type) {
            return;
        }
        let chat_id = item.chat_id.clone();
        if self.chat_session_cache_manager.is_deleting(&chat_id) {
            return;
        }
        let item_id = item.id.clone();
        let ai_chat_item_service = self.ai_chat_item_service.clone();
        tokio::spawn(async move {
            if let Err(error) = ai_chat_item_service.insert(item).await {
                warn!("[ai] 保存ai_chat_item 失败: chat_id=[{}] item_id=[{}] error={}", chat_id, item_id, error);
                return;
            }
            ai_chat_item_service.remove_ids_from_cache(chat_id, &HashSet::from([item_id]));
        });
    }
}
