use crate::model::config::{split_config_list, Config, CONFIRM_PREFIX_SKIP_TOKENS_DEFAULT, CONFIRM_PREFIX_TOOLS_DEFAULT};

#[derive(Debug, Clone)]
pub struct CommandPrefixRule {
    skip_tokens: Vec<String>,
    subcommand_tools: Vec<String>,
}

impl CommandPrefixRule {
    pub fn from_config(config: Option<&Config>) -> Self {
        match config {
            Some(config) => Self { skip_tokens: config.resolve_confirm_prefix_skip_tokens(), subcommand_tools: config.resolve_confirm_prefix_tools() },
            None => Self { skip_tokens: split_config_list(None, CONFIRM_PREFIX_SKIP_TOKENS_DEFAULT), subcommand_tools: split_config_list(None, CONFIRM_PREFIX_TOOLS_DEFAULT) },
        }
    }

    pub fn tokens_of(&self, command: &str) -> Vec<String> {
        let Some(first_line) = command.lines().find(|line| !line.trim().is_empty()) else {
            return Vec::new();
        };
        let mut tokens = first_line.trim().split_whitespace().filter(|token| !token.contains('=') || token.starts_with('-')).skip_while(|token| self.skip_tokens.iter().any(|skip| skip == token));
        let Some(head) = tokens.next() else {
            return Vec::new();
        };
        let head = match head.rsplit(['/', '\\']).next() {
            Some(name) => name.to_ascii_lowercase(),
            None => head.to_ascii_lowercase(),
        };
        let mut out = vec![head];
        out.extend(tokens.map(|token| token.to_ascii_lowercase()));
        out
    }

    pub fn prefix_of(&self, command: &str) -> String {
        let tokens = self.tokens_of(command);
        let Some(head) = tokens.first() else {
            return String::new();
        };
        if !self.subcommand_tools.contains(head) {
            return head.clone();
        }
        match tokens.get(1) {
            Some(sub) if !sub.starts_with('-') => format!("{} {}", head, sub),
            _ => head.clone(),
        }
    }

    pub fn prefix_tokens_of(&self, command: &str) -> Vec<String> {
        self.prefix_of(command).split_whitespace().map(|token| token.to_string()).collect()
    }
}
