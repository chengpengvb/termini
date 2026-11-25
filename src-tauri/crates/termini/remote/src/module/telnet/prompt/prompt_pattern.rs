use regex::Regex;
use summer::tracing::warn;
use crate::model::config_telnet::ConfigTelnet;
use crate::module::telnet::prompt::prompt_text::{extract_base_prompt, extract_prompt_line};
use crate::model::config_telnet::{MORE_PATTERN, MORE_RESPONSE, PASSWORD_PATTERN, PROMPT_ALT_PATTERN, PROMPT_PRI_PATTERN, USERNAME_PATTERN};

#[derive(Debug, Clone)]
pub struct TelnetPromptPattern {
    more: Regex,
    username: Regex,
    password: Regex,
    prompt_pri: Regex,
    prompt_alt: Regex,
    more_response: String,
}

impl TelnetPromptPattern {
    pub fn from_config(config: &ConfigTelnet) -> Self {
        Self {
            more: compile(config.more_pattern.as_deref(), MORE_PATTERN),
            username: compile(config.username_pattern.as_deref(), USERNAME_PATTERN),
            password: compile(config.password_pattern.as_deref(), PASSWORD_PATTERN),
            prompt_pri: compile(config.prompt_pri_pattern.as_deref(), PROMPT_PRI_PATTERN),
            prompt_alt: compile(config.prompt_alt_pattern.as_deref(), PROMPT_ALT_PATTERN),
            more_response: resolve_more_response(config.more_response.as_deref()),
        }
    }

    pub fn more_response(&self) -> &str {
        &self.more_response
    }

    pub fn is_username_prompt(&self, text: &str) -> bool {
        self.match_prompt_line(text, |line| self.username.is_match(line))
    }

    pub fn is_password_prompt(&self, text: &str) -> bool {
        self.match_prompt_line(text, |line| self.password.is_match(line))
    }

    pub fn is_device_prompt(&self, text: &str) -> bool {
        self.match_prompt_line(text, |line| self.prompt_pri.is_match(line) || self.prompt_alt.is_match(line))
    }

    fn match_prompt_line<F: Fn(&str) -> bool>(&self, text: &str, matches: F) -> bool {
        let Some(line) = extract_prompt_line(text) else {
            return false;
        };
        matches(&line)
    }

    pub fn find_more_prompt(&self, text: &str) -> Option<(usize, usize)> {
        self.more.find(text).map(|matched| (matched.start(), matched.end()))
    }

    pub fn is_command_end(&self, prompt_line: &str, text: &str) -> bool {
        let Some(line) = extract_prompt_line(text) else {
            return false;
        };
        if !self.is_device_prompt(&line) {
            return false;
        }
        let expect = extract_base_prompt(prompt_line);
        let base = extract_base_prompt(&line);
        base.starts_with(&expect) || expect.starts_with(&base)
    }
}

fn resolve_more_response(value: Option<&str>) -> String {
    match value {
        Some(text) if !text.is_empty() => text.to_string(),
        _ => MORE_RESPONSE.to_string(),
    }
}

fn compile(value: Option<&str>, fallback: &str) -> Regex {
    let Some(pattern) = value.map(|text| text.trim()).filter(|text| !text.is_empty()) else {
        return Regex::new(fallback).expect("telnet default prompt pattern");
    };
    match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(error) => {
            warn!("[telnet] 正则编译失败，回退默认值 pattern={} fallback={} error={}", pattern, fallback, error);
            Regex::new(fallback).expect("telnet default prompt pattern")
        }
    }
}
