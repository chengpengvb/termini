use anyhow::Result;
use serde::Serialize;
use summer::plugin::Service;
use crate::model::ai_plan_run;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::model::ai_plan_run::AiPlanRun;
use crate::dao::dao_ai_plan_run::AiPlanRunDao;
use crate::service::service_base::BaseService;
use crate::dao::dao_ai_plan_task::AiPlanTaskDao;
use crate::service::service_ai_chat::AiChatService;

const RECENT_RUN_LIMIT: u64 = 200;

#[derive(Debug, Clone, Serialize)]
pub struct RespPlanRun {
    #[serde(flatten)]
    pub run: AiPlanRun,
    pub task_name: String,
}

#[base_service(ai_plan_run::Entity, dao = "ai_plan_run_dao")]
#[derive(Clone, Service)]
pub struct AiPlanRunService {
    #[inject(component)]
    ai_plan_run_dao: AiPlanRunDao,

    #[inject(component)]
    ai_plan_task_dao: AiPlanTaskDao,

    #[inject(component)]
    ai_chat_service: AiChatService,
}

impl AiPlanRunService {
    pub async fn select_recent(&self) -> Result<Vec<RespPlanRun>> {
        let runs = self.ai_plan_run_dao.select_recent(RECENT_RUN_LIMIT).await?;
        self.fill_names(runs).await
    }

    pub async fn select_by_task_id(&self, task_id: String) -> Result<Vec<RespPlanRun>> {
        let runs = self.ai_plan_run_dao.select_by_task_id(&task_id).await?;
        self.fill_names(runs).await
    }

    pub async fn delete_run(&self, run_id: String) -> Result<()> {
        let Some(shared) = self.ai_plan_run_dao.find_by_id(run_id.clone()).await? else {
            return Ok(());
        };
        let chat_id = shared.read().chat_id.clone();
        self.ai_plan_run_dao.delete_by_id(run_id).await?;
        if !chat_id.is_empty() {
            self.ai_chat_service.delete_by_id(chat_id).await?;
        }
        Ok(())
    }

    async fn fill_names(&self, runs: Vec<AiPlanRun>) -> Result<Vec<RespPlanRun>> {
        let mut list = Vec::with_capacity(runs.len());
        for run in runs {
            let task_name = match self.ai_plan_task_dao.find_by_id(run.task_id.clone()).await? {
                Some(task) => task.read().name.clone(),
                None => String::new(),
            };
            list.push(RespPlanRun { run, task_name });
        }
        Ok(list)
    }
}
