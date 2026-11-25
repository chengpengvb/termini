use termini_macros::base_target;
use serde::{Deserialize, Serialize};

pub const RULE_DECISION_ALLOW: i16 = 1;
pub const RULE_DECISION_PROMPT: i16 = 2;
pub const RULE_DECISION_FORBIDDEN: i16 = 3;

#[base_target(table = "command_rule", entity_type = 24)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub pattern: String,
    pub decision: i16,
    pub device_id: String,
    pub created_at: i64,
    pub justification: String,
}

pub type CommandRule = Model;

impl Model {
    pub fn build(pattern: String, decision: i16, device_id: String, justification: String, created_at: i64) -> Self {
        Self { id: String::new(), pattern, decision, device_id, created_at, justification }
    }

    pub fn pattern_tokens(&self) -> Option<Vec<Vec<String>>> {
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&self.pattern).ok()?;
        let mut tokens: Vec<Vec<String>> = Vec::new();
        for item in parsed {
            match item {
                serde_json::Value::String(text) => tokens.push(vec![text.to_lowercase()]),
                serde_json::Value::Array(alternatives) => {
                    let names: Vec<String> = alternatives.into_iter().filter_map(|value| value.as_str().map(|text| text.to_lowercase())).collect();
                    if names.is_empty() {
                        return None;
                    }
                    tokens.push(names);
                }
                _ => return None,
            }
        }
        if tokens.is_empty() {
            return None;
        }
        Some(tokens)
    }

    pub fn pattern_text(&self) -> String {
        match self.pattern_tokens() {
            Some(tokens) => tokens.iter().map(|alternatives| alternatives.join("|")).collect::<Vec<String>>().join(" "),
            None => self.pattern.clone(),
        }
    }
}
