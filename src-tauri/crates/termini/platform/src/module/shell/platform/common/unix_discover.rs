use std::path::PathBuf;
use anyhow::{anyhow, Result};
use crate::module::common::context::context_shell::ShellDescr;

pub fn resolve_shell_description(file_name: &str) -> String {
    match file_name {
        "bash" => "GNU Bash".to_owned(),
        "zsh" => "Z shell (zsh)".to_owned(),
        "fish" => "Fish shell".to_owned(),
        "sh" => "POSIX sh".to_owned(),
        "dash" => "Debian Almquist shell (dash)".to_owned(),
        "ksh" | "ksh93" => "KornShell (ksh)".to_owned(),
        "tcsh" => "TENEX C shell (tcsh)".to_owned(),
        "csh" => "C shell (csh)".to_owned(),
        other => format!("Unix shell ({other})"),
    }
}

pub fn build_exec_args(descr: &ShellDescr, command: &str) -> Result<Vec<String>> {
    let Some(exe) = descr.exe.as_ref() else {
        return Err(anyhow!("shell.exe is not set"));
    };
    let Some(file_name) = exe.file_name().and_then(|name| name.to_str()) else {
        return Err(anyhow!("cannot resolve shell executable name: {}", descr.shell_id));
    };
    match file_name {
        "fish" => Ok(vec!["-c".to_owned(), command.to_owned()]),
        _ => Ok(vec!["-lc".to_owned(), command.to_owned()]),
    }
}

pub fn user_default_shell() -> Option<PathBuf> {
    let value = std::env::var("SHELL").ok()?;
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    if !path.is_file() {
        return None;
    }
    Some(path)
}

pub fn read_etc_shells() -> Vec<PathBuf> {
    let content = std::fs::read_to_string("/etc/shells").ok();
    let Some(content) = content else {
        return Vec::new();
    };

    let mut result: Vec<PathBuf> = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let path = PathBuf::from(line);
        if path.is_file() {
            result.push(path);
        }
    }
    result
}
