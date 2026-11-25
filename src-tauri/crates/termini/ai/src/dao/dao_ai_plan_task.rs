use summer::plugin::Service;
use termini_macros::base_dao;
use crate::model::ai_plan_task;

#[base_dao(ai_plan_task::Entity, sort = 120, phase = 30, cache_all = true)]
#[derive(Clone, Service)]
pub struct AiPlanTaskDao {}
