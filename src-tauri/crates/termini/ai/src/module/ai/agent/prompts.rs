pub const LEAD_MAIN_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/main_system.md");
pub const LEAD_AUDIT_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/audit_system.md");
pub const LEAD_ALERT_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/alert_system.md");
pub const LEAD_ALERT_USER: &str = include_str!("../../../../resource/prompts/lead/alert_user.md");
pub const LEAD_ALERT_GRANT_ANALYZE: &str = include_str!("../../../../resource/prompts/lead/alert_grant_analyze.md");
pub const LEAD_ALERT_GRANT_REPAIR: &str = include_str!("../../../../resource/prompts/lead/alert_grant_repair.md");
pub const LEAD_DISCOVER_USER: &str = include_str!("../../../../resource/prompts/lead/discover_user.md");
pub const LEAD_DISCOVER_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/discover_system.md");
pub const LEAD_OUTPUT_LANGUAGE: &str = include_str!("../../../../resource/prompts/lead/output_language.md");
pub const LEAD_OUTPUT_LANGUAGE_FIXED: &str = include_str!("../../../../resource/prompts/lead/output_language_fixed.md");
pub const LEAD_ROUTER_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/router_system.md");
pub const LEAD_ROUTER_TITLE: &str = include_str!("../../../../resource/prompts/lead/router_title.md");
pub const LEAD_DIRECT_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/direct_system.md");
pub const LEAD_FOLD_USER: &str = include_str!("../../../../resource/prompts/lead/fold_user.md");
pub const LEAD_FOLD_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/fold_system.md");
pub const LEAD_HOT_MEMORY_TRIMMED: &str = include_str!("../../../../resource/prompts/lead/hot_memory_trimmed.md");
pub const LEAD_SHELL_SUPERVISE_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/shell_supervise_system.md");
pub const LEAD_SKILL_EXTRACT_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/skill_extract_system.md");
pub const LEAD_SKILL_EXTRACT_USER: &str = include_str!("../../../../resource/prompts/lead/skill_extract_user.md");
pub const LEAD_IMAGE_READ_SYSTEM: &str = include_str!("../../../../resource/prompts/lead/image_read_system.md");
pub const COMMAND_SUGGEST_SYSTEM: &str = include_str!("../../../../resource/prompts/suggest/command_suggest_system.md");
pub const COMMAND_SUGGEST_USER: &str = include_str!("../../../../resource/prompts/suggest/command_suggest_user.md");

use rig::completion::Message;
use rig::message::UserContent;
use crate::model::config::SystemLanguage;

pub fn output_language_text(language: SystemLanguage) -> String {
    render(LEAD_OUTPUT_LANGUAGE, &[("language", language.label())])
}

pub fn output_language_fixed_text(language: &str) -> String {
    render(LEAD_OUTPUT_LANGUAGE_FIXED, &[("language", language)])
}

pub fn output_language_addons(messages: &[Message]) -> Vec<Message> {
    messages.iter().filter(|message| output_language_of(message).is_some()).cloned().collect()
}

pub fn output_language_addon_text(messages: &[Message]) -> Option<String> {
    messages.iter().find_map(|message| output_language_of(message).map(str::to_owned))
}

fn output_language_of(message: &Message) -> Option<&str> {
    let Message::User { content } = message else {
        return None;
    };
    content.iter().find_map(|part| match part {
        UserContent::Text(text) if text.text.starts_with("[输出语言]") => Some(text.text.as_str()),
        _ => None,
    })
}

pub fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (key, value) in vars {
        let placeholder = format!("{{{{{key}}}}}");
        out = out.replace(&placeholder, value);
    }
    out
}
