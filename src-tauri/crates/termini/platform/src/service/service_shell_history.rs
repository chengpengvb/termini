use std::path::PathBuf;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::{AppContext, BaseApp};

const HISTORY_MAX_ITEMS: usize = 100;
const HISTORY_READ_LINES: usize = 300;

#[derive(Clone, Service)]
pub struct ShellHistoryService {}

#[async_trait]
impl BaseApp for ShellHistoryService {
    fn name(&self) -> &'static str {
        "ShellHistoryService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl ShellHistoryService {
    pub fn get_cmd_history(&self, shell_id: &str) -> Vec<String> {
        let Some(path) = resolve_history_path(shell_id) else {
            return Vec::new();
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text.lines().collect();
        let start = lines.len().saturating_sub(HISTORY_READ_LINES);
        let mut history: Vec<String> = Vec::new();
        for line in &lines[start..] {
            let Some(command) = parse_line(shell_id, line) else {
                continue;
            };
            if command.is_empty() {
                continue;
            }
            if history.last().map(String::as_str) == Some(command.as_str()) {
                continue;
            }
            history.push(command);
        }
        let overflow = history.len().saturating_sub(HISTORY_MAX_ITEMS);
        history.drain(..overflow);
        history
    }
}

fn parse_line(shell_id: &str, line: &str) -> Option<String> {
    let trimmed = line.trim();
    if shell_id.ends_with(".zsh") {
        if let Some(rest) = trimmed.strip_prefix(": ") {
            return rest.split_once(';').map(|(_, cmd)| cmd.trim().to_string());
        }
        return Some(trimmed.to_string());
    }
    if shell_id.ends_with(".fish") {
        return trimmed.strip_prefix("- cmd: ").map(|cmd| cmd.trim().to_string());
    }
    Some(trimmed.to_string())
}

fn resolve_history_path(shell_id: &str) -> Option<PathBuf> {
    let kind = shell_id.rsplit('.').next().unwrap_or_default();
    match kind {
        "powershell" | "pwsh" => {
            let appdata = std::env::var("APPDATA").ok()?;
            Some(PathBuf::from(appdata).join("Microsoft").join("Windows").join("PowerShell").join("PSReadLine").join("ConsoleHost_history.txt"))
        }
        "bash" | "git_bash" | "sh" => Some(home_dir()?.join(".bash_history")),
        "zsh" => Some(home_dir()?.join(".zsh_history")),
        "fish" => Some(home_dir()?.join(".local").join("share").join("fish").join("fish_history")),
        _ => None,
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}
