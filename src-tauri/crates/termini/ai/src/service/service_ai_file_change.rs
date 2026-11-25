use std::sync::Arc;
use summer::tracing::error;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use crate::model::ai_file_change;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use summer::plugin::LazyComponent;
use crate::util::time_util::now_millis;
use crate::web::ws::ws_manager::WsManager;
use crate::dao::dao_ai_file_change::AiFileChangeDao;
use crate::module::ai::shared::AgentRemoteClientPool;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::resp::resp_sftp::RespSftpDeleteProgressHandler;
use crate::model::resp::resp_ai_file_change::RespAiFileChangeView;
use crate::module::common::dto::file_entry::{FileKind, PlannedDelete};
use crate::model::resp::event::resp_event_ai_file_change::RespEventAiFileChange;
use crate::module::ai::tools::common::tool_sftp_search_replace::ToolSftpSearchReplace;
use crate::model::ai_file_change::{AiFileChange, FILE_CHANGE_STATE_ACCEPTED, FILE_CHANGE_STATE_PENDING, FILE_CHANGE_STATE_REVERTED};

#[base_service(ai_file_change::Entity, dao = "ai_file_change_dao")]
#[derive(Clone, Service)]
pub struct AiFileChangeService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    ai_file_change_dao: AiFileChangeDao,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    agent_remote_client_pool: LazyComponent<AgentRemoteClientPool>,
}

impl AiFileChangeService {
    pub async fn record(&self, req_id: &str, device_id: Option<String>, path: String, before_text: Option<String>, after_text: String) {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            error!("[file] 记录变更失败：未知 req_id={} path={}", req_id, path);
            return;
        };
        let chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        self.chat_session_cache_manager.add_changed_path(req_id, &path);
        let entity =
            AiFileChange { id: uuid(), chat_id: chat_id.clone(), req_id: req_id.to_string(), device_id, path, before_text, after_text, state: FILE_CHANGE_STATE_PENDING, timestamp: now_millis() };
        if let Err(error) = self.ai_file_change_dao.insert(entity).await {
            error!("[file] 记录变更入库失败 req_id={} error={:#}", req_id, error);
            return;
        }
        self.emit(chat_id);
    }

    pub async fn list_view(&self, chat_id: String) -> Result<Vec<RespAiFileChangeView>> {
        let rows = self.ai_file_change_dao.find_by_chat_id(&chat_id).await?;
        let mut order: Vec<String> = Vec::new();
        let mut groups: HashMap<String, Vec<AiFileChange>> = HashMap::new();
        for row in rows {
            let key = Self::group_key(&row.device_id, &row.path);
            if !groups.contains_key(&key) {
                order.push(key.clone());
            }
            groups.entry(key).or_default().push(row);
        }
        let mut views = Vec::with_capacity(order.len());
        for key in order {
            let Some(rows) = groups.remove(&key) else {
                continue;
            };
            views.push(Self::build_view(rows).await);
        }
        Ok(views)
    }

    pub async fn accept(&self, chat_id: String, device_id: Option<String>, path: String) -> Result<()> {
        let rows = self.pending_rows(&chat_id, &device_id, &path).await?;
        for mut row in rows {
            row.state = FILE_CHANGE_STATE_ACCEPTED;
            self.ai_file_change_dao.update(row).await?;
        }
        self.emit(chat_id);
        Ok(())
    }

    pub async fn revert(&self, chat_id: String, device_id: Option<String>, path: String) -> Result<()> {
        let rows = self.pending_rows(&chat_id, &device_id, &path).await?;
        let baseline = rows.first().and_then(|row| row.before_text.clone());
        match &device_id {
            None => Self::revert_local(&path, baseline).await?,
            Some(device) => self.revert_remote(device, &path, baseline).await?,
        }
        for mut row in rows {
            row.state = FILE_CHANGE_STATE_REVERTED;
            self.ai_file_change_dao.update(row).await?;
        }
        self.emit(chat_id);
        Ok(())
    }

    pub async fn delete_by_chat_id(&self, chat_id: String) -> Result<u64> {
        self.ai_file_change_dao.delete_by_chat_id(chat_id).await
    }

    async fn pending_rows(&self, chat_id: &str, device_id: &Option<String>, path: &str) -> Result<Vec<AiFileChange>> {
        let rows = self.ai_file_change_dao.find_by_chat_id(chat_id).await?;
        let pending: Vec<AiFileChange> = rows.into_iter().filter(|row| row.state == FILE_CHANGE_STATE_PENDING && row.device_id == *device_id && row.path == path).collect();
        if pending.is_empty() {
            return Err(anyhow!("the file has no pending changes: {}", path));
        }
        Ok(pending)
    }

    async fn revert_local(path: &str, baseline: Option<String>) -> Result<()> {
        match baseline {
            Some(text) => tokio::fs::write(path, text).await.map_err(|error| anyhow!("{}: {}", path, error)),
            None => tokio::fs::remove_file(path).await.map_err(|error| anyhow!("{}: {}", path, error)),
        }
    }

    async fn revert_remote(&self, device_id: &str, path: &str, baseline: Option<String>) -> Result<()> {
        let pool = self.agent_remote_client_pool.get().map_err(|error| anyhow!(error.to_string()))?;
        let pool_key = format!("file-change-revert:{}", uuid());
        let client = pool.get_or_open_sftp(&pool_key, device_id).await.map_err(|error| anyhow!(error.to_string()))?;
        let result = match baseline {
            Some(text) => ToolSftpSearchReplace::write_text(client.as_ref(), path, &text).await.map_err(|error| anyhow!(error.to_string())),
            None => client.delete(&[PlannedDelete { path: path.to_string(), kind: FileKind::File }], None, Self::noop_delete_handler()).await,
        };
        pool.release(&pool_key);
        result
    }

    fn noop_delete_handler() -> RespSftpDeleteProgressHandler {
        Arc::new(|_| Box::pin(async { Ok(()) }))
    }

    async fn build_view(rows: Vec<AiFileChange>) -> RespAiFileChangeView {
        let pending: Vec<&AiFileChange> = rows.iter().filter(|row| row.state == FILE_CHANGE_STATE_PENDING).collect();
        let scope: Vec<&AiFileChange> = if pending.is_empty() { rows.iter().collect() } else { pending };
        let first = scope[0];
        let last = scope[scope.len() - 1];
        let state = if first.state == FILE_CHANGE_STATE_PENDING { FILE_CHANGE_STATE_PENDING } else { last.state };
        let baseline_text = first.before_text.clone();
        let current_text = Self::resolve_current_text(&first.device_id, &first.path, state, &baseline_text, last).await;
        RespAiFileChangeView {
            chat_id: first.chat_id.clone(),
            device_id: first.device_id.clone(),
            path: first.path.clone(),
            state,
            edit_count: scope.len(),
            baseline_text,
            current_text,
            timestamp: last.timestamp,
        }
    }

    async fn resolve_current_text(device_id: &Option<String>, path: &str, state: i16, baseline_text: &Option<String>, last: &AiFileChange) -> Option<String> {
        if device_id.is_none() {
            return tokio::fs::read_to_string(path).await.ok();
        }
        if state == FILE_CHANGE_STATE_REVERTED {
            return baseline_text.clone();
        }
        Some(last.after_text.clone())
    }

    fn group_key(device_id: &Option<String>, path: &str) -> String {
        match device_id {
            Some(device) => format!("{}:{}", device, path),
            None => format!("local:{}", path),
        }
    }

    fn emit(&self, chat_id: String) {
        self.ws_manager.broadcast_all(&RespEventAiFileChange::OnChanged(chat_id).to_resp());
    }
}
