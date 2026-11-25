use anyhow::Result;
use summer::plugin::Service;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use crate::model::ai_chat_2_project;
use crate::web::ws::ws_manager::WsManager;
use crate::dao::dao_ai_project::AiProjectDao;
use crate::model::resp::resp_model::ResultCode;
use crate::model::ai_chat_2_project::AiChat2Project;
use crate::dao::dao_ai_chat_2_project::AiChat2ProjectDao;
use crate::model::resp::event::resp_event_ai_chat::RespEventAiChat;
use crate::model::resp::event::resp_event_ai_project::RespEventAiProject;

#[base_service(ai_chat_2_project::Entity, dao = "ai_chat_2_project_dao")]
#[derive(Clone, Service)]
pub struct AiChat2ProjectService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    ai_project_dao: AiProjectDao,

    #[inject(component)]
    ai_chat_2_project_dao: AiChat2ProjectDao,
}

impl AiChat2ProjectService {
    pub fn list(&self) -> Vec<AiChat2Project> {
        self.ai_chat_2_project_dao.cache_map().iter().map(|entry| entry.value().read().clone()).collect()
    }

    pub async fn set_project(&self, chat_id: String, project_id: String) -> Result<()> {
        if self.ai_project_dao.cache_map().get(&project_id).is_none() {
            return Err(ResultCode::ProjectNotFound.into());
        }
        let existing = self.find_by_chat(&chat_id);
        for link in &existing {
            if link.project_id == project_id {
                continue;
            }
            self.ai_chat_2_project_dao.delete_by_id(link.id.clone()).await?;
            self.emit_project(link.project_id.clone());
        }
        if existing.iter().any(|link| link.project_id == project_id) {
            return Ok(());
        }
        let entity = AiChat2Project { id: uuid(), chat_id: chat_id.clone(), project_id: project_id.clone() };
        self.ai_chat_2_project_dao.insert(entity).await?;
        self.emit_project(project_id);
        self.emit_chat(chat_id);
        Ok(())
    }

    pub async fn unset(&self, chat_id: String) -> Result<()> {
        let existing = self.find_by_chat(&chat_id);
        if existing.is_empty() {
            return Ok(());
        }
        for link in &existing {
            self.ai_chat_2_project_dao.delete_by_id(link.id.clone()).await?;
            self.emit_project(link.project_id.clone());
        }
        self.emit_chat(chat_id);
        Ok(())
    }

    pub async fn delete_by_chat(&self, chat_id: &str) -> Result<()> {
        let existing = self.find_by_chat(chat_id);
        for link in existing {
            self.ai_chat_2_project_dao.delete_by_id(link.id).await?;
        }
        Ok(())
    }

    fn find_by_chat(&self, chat_id: &str) -> Vec<AiChat2Project> {
        self.ai_chat_2_project_dao.cache_map().iter().filter(|entry| entry.value().read().chat_id == chat_id).map(|entry| entry.value().read().clone()).collect()
    }

    fn emit_project(&self, project_id: String) {
        self.ws_manager.broadcast_all(&RespEventAiProject::OnChanged(project_id).to_resp());
    }

    fn emit_chat(&self, chat_id: String) {
        self.ws_manager.broadcast_all(&RespEventAiChat::OnChanged(chat_id).to_resp());
    }
}
