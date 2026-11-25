use summer::plugin::Service;
use crate::model::command_rule;
use termini_macros::base_service;
use crate::util::time_util::now_millis;
use crate::service::service_base::BaseService;
use crate::dao::dao_command_rule::CommandRuleDao;
use crate::model::command_rule::{CommandRule, RULE_DECISION_ALLOW, RULE_DECISION_FORBIDDEN, RULE_DECISION_PROMPT};

#[derive(Debug, Clone)]
pub struct RuleHit {
    pub decision: i16,
    pub rule_id: String,
    pub justification: String,
    pub matched_prefix: String,
}

#[base_service(command_rule::Entity, dao = "command_rule_dao")]
#[derive(Clone, Service)]
pub struct CommandRuleService {
    #[inject(component)]
    command_rule_dao: CommandRuleDao,
}

impl CommandRuleService {
    pub fn list(&self) -> Vec<CommandRule> {
        self.command_rule_dao.list_cached()
    }

    pub fn evaluate(&self, command_tokens: &[String], device_ids: &[String]) -> Option<RuleHit> {
        let mut best: Option<RuleHit> = None;
        for rule in self.command_rule_dao.list_cached() {
            if !rule.device_id.is_empty() && !device_ids.iter().any(|device_id| device_id == &rule.device_id) {
                continue;
            }
            let Some(pattern) = rule.pattern_tokens() else {
                continue;
            };
            if !Self::matches(&pattern, command_tokens) {
                continue;
            }
            let hit = RuleHit { decision: rule.decision, rule_id: rule.id.clone(), matched_prefix: command_tokens[..pattern.len()].join(" "), justification: rule.justification.clone() };
            let stricter = best.as_ref().is_none_or(|current| hit.decision > current.decision);
            if stricter {
                best = Some(hit);
            }
        }
        best
    }

    pub async fn create(&self, pattern_tokens: Vec<String>, decision: i16, device_id: String, justification: String) -> anyhow::Result<CommandRule> {
        if pattern_tokens.is_empty() {
            return Err(anyhow::anyhow!("rule pattern must not be empty"));
        }
        if !matches!(decision, RULE_DECISION_ALLOW | RULE_DECISION_PROMPT | RULE_DECISION_FORBIDDEN) {
            return Err(anyhow::anyhow!("invalid rule decision: {}", decision));
        }
        let pattern = serde_json::to_string(&pattern_tokens.iter().map(|token| Self::pattern_value(token)).collect::<Vec<serde_json::Value>>())?;
        let entity = CommandRule::build(pattern, decision, device_id, justification.trim().to_string(), now_millis());
        let saved = self.insert(entity).await?;
        let rule = saved.read().clone();
        Ok(rule)
    }

    pub async fn remember_allow(&self, pattern_tokens: Vec<String>, device_id: String) -> anyhow::Result<()> {
        let lowered: Vec<String> = pattern_tokens.iter().map(|token| token.to_lowercase()).collect();
        let exists = self.command_rule_dao.list_cached().into_iter().any(|rule| {
            rule.device_id == device_id
                && rule.decision == RULE_DECISION_ALLOW
                && rule.pattern_tokens().is_some_and(|tokens| tokens.iter().map(|alternatives| alternatives.join("|")).collect::<Vec<String>>() == lowered)
        });
        if exists {
            return Ok(());
        }
        self.create(lowered, RULE_DECISION_ALLOW, device_id, String::new()).await?;
        Ok(())
    }

    pub async fn remove(&self, id: String) -> anyhow::Result<u64> {
        self.delete_by_id(id).await
    }

    fn pattern_value(token: &str) -> serde_json::Value {
        let alternatives: Vec<String> = token.split('|').map(|item| item.trim().to_lowercase()).filter(|item| !item.is_empty()).collect();
        if alternatives.len() == 1 {
            return serde_json::Value::String(alternatives[0].clone());
        }
        serde_json::Value::Array(alternatives.into_iter().map(serde_json::Value::String).collect())
    }

    fn matches(pattern: &[Vec<String>], command_tokens: &[String]) -> bool {
        if pattern.len() > command_tokens.len() {
            return false;
        }
        pattern.iter().zip(command_tokens.iter()).all(|(alternatives, token)| alternatives.iter().any(|candidate| candidate == token))
    }
}
