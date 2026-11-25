use summer::plugin::Service;
use termini_macros::base_dao;
use crate::model::ai_plan_run;
use crate::dao::dao_base::BaseDao;
use crate::model::ai_plan_run::AiPlanRun;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

#[base_dao(ai_plan_run::Entity, sort = 120, phase = 30, cache_all = false)]
#[derive(Clone, Service)]
pub struct AiPlanRunDao {}

impl AiPlanRunDao {
    pub async fn select_by_task_id(&self, task_id: &str) -> anyhow::Result<Vec<AiPlanRun>> {
        let conn = self.get_conn()?;
        let rows = ai_plan_run::Entity::find().filter(ai_plan_run::Column::TaskId.eq(task_id)).order_by_desc(ai_plan_run::Column::StartedAt).all(&conn).await?;
        Ok(rows)
    }

    pub async fn select_recent(&self, limit: u64) -> anyhow::Result<Vec<AiPlanRun>> {
        let conn = self.get_conn()?;
        let rows = ai_plan_run::Entity::find().order_by_desc(ai_plan_run::Column::StartedAt).limit(limit).all(&conn).await?;
        Ok(rows)
    }

    pub async fn delete_by_task_id(&self, task_id: &str) -> anyhow::Result<u64> {
        let conn = self.get_conn()?;
        let result = ai_plan_run::Entity::delete_many().filter(ai_plan_run::Column::TaskId.eq(task_id)).exec(&conn).await?;
        Ok(result.rows_affected)
    }
}
