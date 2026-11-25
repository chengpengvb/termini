use std::path::PathBuf;
use crate::module::shell::platform::common::unix_discover;
use crate::module::common::context::context_shell::ShellDescr;
use crate::module::shell::platform::shell_discover::ShellDiscover;

pub struct ShellDiscoverLinux;

impl ShellDiscoverLinux {
    pub fn new() -> Self {
        Self
    }

    fn shell_descriptor(&self, exe: PathBuf) -> ShellDescr {
        let file_name = exe.file_name().and_then(|s| s.to_str()).unwrap_or("shell");
        let shell_id = format!("linux.{file_name}");
        let description = unix_discover::resolve_shell_description(file_name);
        ShellDescr { shell_id, description, exe: Some(exe), args: Vec::new(), env: Vec::new(), supports_osc633: true }
    }
}

impl ShellDiscover for ShellDiscoverLinux {
    fn discover(&self) -> Vec<ShellDescr> {
        let mut seen: Vec<PathBuf> = Vec::new();
        let mut result: Vec<ShellDescr> = Vec::new();

        if let Some(default_shell) = unix_discover::user_default_shell() {
            seen.push(default_shell.clone());
            result.push(self.shell_descriptor(default_shell));
        }

        for path in unix_discover::read_etc_shells() {
            if seen.iter().any(|existing| existing == &path) {
                continue;
            }
            seen.push(path.clone());
            result.push(self.shell_descriptor(path));
        }

        result
    }

    fn build_exec_args(&self, descr: &ShellDescr, command: &str) -> anyhow::Result<Vec<String>> {
        unix_discover::build_exec_args(descr, command)
    }
}
