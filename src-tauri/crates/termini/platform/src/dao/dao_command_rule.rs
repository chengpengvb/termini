use summer::plugin::Service;
use termini_macros::base_dao;
use crate::model::command_rule;
use crate::dao::dao_base::BaseDao;
use crate::model::command_rule::CommandRule;

#[base_dao(command_rule::Entity, sort = 120, phase = 30, cache_all = true)]
#[derive(Clone, Service)]
pub struct CommandRuleDao {}

impl CommandRuleDao {
    pub fn list_cached(&self) -> Vec<CommandRule> {
        let mut rules: Vec<CommandRule> = self.cache_map().iter().map(|entry| entry.value().read().clone()).collect();
        rules.sort_by_key(|rule| rule.created_at);
        rules
    }
}
