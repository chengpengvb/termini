use crate::model::ai_skill;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::dao::dao_ai_skill::AiSkillDao;
use crate::model::dto::shared_ptr::Shared;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::model::ai_skill::{AiSkill, AI_SKILL_DESCRIPTION_MAX_LEN};

#[base_service(ai_skill::Entity, dao = "ai_skill_dao", sync = 12)]
#[derive(Clone, Service)]
pub struct AiSkillService {
    #[inject(component)]
    ai_skill_dao: AiSkillDao,
}

#[async_trait]
impl BaseService for AiSkillService {
    type Entity = ai_skill::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.ai_skill_dao.clone()))
    }
}

impl AiSkillService {
    pub async fn create_skill(&self, mut skill: AiSkill) -> anyhow::Result<Shared<AiSkill>> {
        self.normalize_skill(&mut skill)?;
        self.ai_skill_dao.insert(skill).await
    }

    pub async fn update_skill(&self, mut skill: AiSkill) -> anyhow::Result<Shared<AiSkill>> {
        self.normalize_skill(&mut skill)?;
        self.ai_skill_dao.update(skill).await
    }

    fn normalize_skill(&self, skill: &mut AiSkill) -> anyhow::Result<()> {
        let name = skill.name.trim().to_string();
        if name.is_empty() {
            return Err(ResultCode::SkillNameEmpty.into());
        }
        let content = skill.content.trim().to_string();
        if content.is_empty() {
            return Err(ResultCode::SkillContentEmpty.into());
        }
        let description = skill.description.trim().to_string();
        if description.chars().count() > AI_SKILL_DESCRIPTION_MAX_LEN {
            return Err(ResultCode::SkillDescriptionTooLong.into());
        }
        skill.name = name;
        skill.description = description;
        skill.content = content;
        Ok(())
    }
}
