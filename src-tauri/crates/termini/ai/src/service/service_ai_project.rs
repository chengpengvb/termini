use anyhow::Result;
use summer::plugin::Service;
use crate::model::ai_project;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use crate::util::time_util::now_millis;
use crate::model::ai_project::AiProject;
use crate::model::dto::shared_ptr::Shared;
use crate::web::ws::ws_manager::WsManager;
use crate::dao::dao_ai_project::AiProjectDao;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::dao::dao_ai_chat_2_project::AiChat2ProjectDao;
use crate::model::resp::event::resp_event_ai_project::RespEventAiProject;

const NAME_MAX_CHARS: usize = 40;

#[base_service(ai_project::Entity, dao = "ai_project_dao")]
#[derive(Clone, Service)]
pub struct AiProjectService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    ai_project_dao: AiProjectDao,

    #[inject(component)]
    ai_chat_2_project_dao: AiChat2ProjectDao,
}

impl AiProjectService {
    pub fn list(&self) -> Vec<AiProject> {
        let mut list: Vec<AiProject> = self.ai_project_dao.cache_map().iter().map(|entry| entry.value().read().clone()).collect();
        list.sort_by(|left, right| left.sort.cmp(&right.sort).then_with(|| right.timestamp.cmp(&left.timestamp)));
        list
    }

    pub async fn create(&self, name: String) -> Result<Shared<AiProject>> {
        let name = Self::normalize_name(&name)?;
        let sort = self.ai_project_dao.cache_map().len() as i16;
        let entity = AiProject { id: uuid(), name, sort, timestamp: now_millis() };
        let saved = self.ai_project_dao.insert(entity).await?;
        let project_id = saved.read().id.clone();
        self.emit(RespEventAiProject::OnChanged(project_id));
        Ok(saved)
    }

    pub async fn rename(&self, id: String, name: String) -> Result<()> {
        let Some(shared) = self.find_by_id(id.clone()).await? else {
            return Err(ResultCode::ProjectNotFound.into());
        };
        let name = Self::normalize_name(&name)?;
        let mut project = shared.read().clone();
        project.name = name;
        self.update(project).await?;
        self.emit(RespEventAiProject::OnChanged(id));
        Ok(())
    }

    pub async fn remove(&self, id: String) -> Result<()> {
        let related: Vec<String> = self.ai_chat_2_project_dao.cache_map().iter().filter(|entry| entry.value().read().project_id == id).map(|entry| entry.value().read().id.clone()).collect();
        for link_id in related {
            self.ai_chat_2_project_dao.delete_by_id(link_id).await?;
        }
        self.delete_by_id(id.clone()).await?;
        self.emit(RespEventAiProject::OnDeleted(id));
        Ok(())
    }

    fn normalize_name(name: &str) -> Result<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ResultCode::ProjectNameEmpty.into());
        }
        let chars: Vec<char> = trimmed.chars().collect();
        if chars.len() <= NAME_MAX_CHARS {
            return Ok(trimmed.to_string());
        }
        Ok(chars.into_iter().take(NAME_MAX_CHARS).collect())
    }

    fn emit(&self, event: RespEventAiProject) {
        self.ws_manager.broadcast_all(&event.to_resp());
    }
}
