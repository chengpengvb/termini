use anyhow::{bail, Result};
use summer::plugin::Service;
use crate::model::ai_plan_task;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::time_util::now_millis;
use crate::model::dto::shared_ptr::Shared;
use crate::web::ws::ws_manager::WsManager;
use crate::dao::dao_ai_plan_run::AiPlanRunDao;
use crate::model::resp::resp_model::ResultCode;
use crate::dao::dao_ai_plan_task::AiPlanTaskDao;
use crate::model::resp::event::resp_event_ai_plan::RespEventAiPlan;
use crate::model::ai_plan_task::{PLAN_CYCLE_EVERY_DAY, PLAN_SCHEDULE_CYCLE, PLAN_SCHEDULE_INTERVAL, PLAN_SCHEDULE_ONCE};

#[base_service(ai_plan_task::Entity, dao = "ai_plan_task_dao")]
#[derive(Clone, Service)]
pub struct AiPlanTaskService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    ai_plan_run_dao: AiPlanRunDao,

    #[inject(component)]
    ai_plan_task_dao: AiPlanTaskDao,
}

impl AiPlanTaskService {
    pub async fn save_task(&self, mut task: ai_plan_task::Model) -> Result<Shared<ai_plan_task::Model>> {
        Self::verify(&task)?;
        task.updated_at = now_millis();
        let saved = if task.id.is_empty() { self.ai_plan_task_dao.insert(task).await? } else { self.ai_plan_task_dao.update(task).await? };
        self.emit_changed(&saved.read().id);
        Ok(saved)
    }

    pub async fn delete_task(&self, id: String) -> Result<()> {
        self.ai_plan_run_dao.delete_by_task_id(&id).await?;
        self.ai_plan_task_dao.delete_by_id(id.clone()).await?;
        self.emit_changed(&id);
        Ok(())
    }

    pub async fn mark_run(&self, id: String, run_at: i64) -> Result<()> {
        let Some(shared) = self.ai_plan_task_dao.find_by_id(id).await? else {
            bail!("plan task not found");
        };
        let mut task = shared.read().clone();
        task.last_run_at = run_at;
        self.ai_plan_task_dao.update(task).await?;
        Ok(())
    }

    pub fn emit_changed(&self, task_id: &str) {
        self.ws_manager.broadcast_all(&RespEventAiPlan::OnChanged(task_id.to_string()).to_resp());
    }

    fn verify(task: &ai_plan_task::Model) -> Result<()> {
        if task.name.trim().is_empty() {
            return Err(ResultCode::NameEmpty.into());
        }
        if task.prompt.trim().is_empty() {
            bail!("prompt must not be empty");
        }
        if task.ai_provide_id.trim().is_empty() {
            bail!("please select an execution model");
        }
        match task.schedule_kind {
            PLAN_SCHEDULE_CYCLE => {
                if task.cycle_day != PLAN_CYCLE_EVERY_DAY && !(1..=7).contains(&task.cycle_day) {
                    bail!("invalid cycle weekday: {}", task.cycle_day);
                }
                if !(0..24 * 60).contains(&task.time_minute) {
                    bail!("invalid execution time: {}", task.time_minute);
                }
            }
            PLAN_SCHEDULE_INTERVAL => {
                if !(1..=24 * 30).contains(&task.interval_hours) {
                    bail!("invalid interval hours: {}", task.interval_hours);
                }
            }
            PLAN_SCHEDULE_ONCE => {
                if task.once_at <= 0 {
                    bail!("invalid one-time execution time");
                }
            }
            other => bail!("unknown schedule kind: {}", other),
        }
        Ok(())
    }
}
