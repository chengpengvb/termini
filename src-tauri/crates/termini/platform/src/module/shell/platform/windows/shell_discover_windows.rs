use winreg::RegKey;
use std::path::{Path, PathBuf};
use crate::module::common::context::context_shell::ShellDescr;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use crate::module::shell::platform::shell_discover::{unknown_exec_shell, ShellDiscover};

pub struct ShellDiscoverWindows;

impl ShellDiscoverWindows {
    pub fn new() -> Self {
        Self
    }

    fn system_root(&self) -> PathBuf {
        if let Ok(value) = std::env::var("SystemRoot") {
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
        PathBuf::from("C:\\Windows")
    }

    fn discover_builtin(&self) -> Vec<ShellDescr> {
        let system_root = self.system_root();
        let mut list: Vec<ShellDescr> = Vec::new();
        let ps_path = system_root.join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe");
        if ps_path.exists() {
            list.push(ShellDescr {
                shell_id: "windows.powershell".to_owned(),
                description: "Windows PowerShell".to_owned(),
                exe: Some(ps_path),
                args: vec!["-NoLogo".to_owned()],
                env: Vec::new(),
                supports_osc633: true,
            });
        }
        let cmd_path = system_root.join("System32").join("cmd.exe");
        if cmd_path.exists() {
            list.push(ShellDescr {
                shell_id: "windows.cmd".to_owned(),
                description: "Windows Command Prompt (cmd)".to_owned(),
                exe: Some(cmd_path),
                args: Vec::new(),
                env: Vec::new(),
                supports_osc633: false,
            });
        }
        list
    }

    fn discover_optional(&self) -> Vec<ShellDescr> {
        let mut list: Vec<ShellDescr> = Vec::new();
        if let Some(pwsh) = self.locate_pwsh() {
            list.push(ShellDescr {
                shell_id: "windows.pwsh".to_owned(),
                description: "PowerShell Core (pwsh)".to_owned(),
                exe: Some(pwsh),
                args: vec!["-NoLogo".to_owned()],
                env: Vec::new(),
                supports_osc633: true,
            });
        }
        if let Some(wsl) = self.locate_wsl() {
            list.push(ShellDescr {
                shell_id: "windows.wsl".to_owned(),
                description: "Windows Subsystem for Linux (WSL)".to_owned(),
                exe: Some(wsl),
                args: Vec::new(),
                env: Vec::new(),
                supports_osc633: true,
            });
        }
        if let Some(git_bash) = self.locate_git_bash() {
            list.push(ShellDescr {
                shell_id: "windows.git_bash".to_owned(),
                description: "Git Bash".to_owned(),
                exe: Some(git_bash),
                args: vec!["--login".to_owned(), "-i".to_owned()],
                env: Vec::new(),
                supports_osc633: true,
            });
        }
        list
    }

    fn locate_pwsh(&self) -> Option<PathBuf> {
        if let Some(path) = self.registry_app_path("pwsh.exe") {
            return Some(path);
        }
        for install_dir in self.registry_powershell_core_install_dirs() {
            let exe = install_dir.join("pwsh.exe");
            if exe.is_file() {
                return Some(exe);
            }
        }
        self.which_in_path("pwsh.exe")
    }

    fn locate_wsl(&self) -> Option<PathBuf> {
        let path = self.system_root().join("System32").join("wsl.exe");
        if path.is_file() {
            return Some(path);
        }
        None
    }

    fn locate_git_bash(&self) -> Option<PathBuf> {
        if let Some(install_dir) = self.registry_git_install_path() {
            if let Some(bash) = self.bash_under_git_install(&install_dir) {
                return Some(bash);
            }
        }
        if let Some(git_exe) = self.registry_app_path("git.exe") {
            if let Some(bash) = self.bash_from_git_exe(&git_exe) {
                return Some(bash);
            }
        }
        if let Some(git_exe) = self.which_in_path("git.exe") {
            if let Some(bash) = self.bash_from_git_exe(&git_exe) {
                return Some(bash);
            }
        }
        None
    }

    fn registry_app_path(&self, exe_name: &str) -> Option<PathBuf> {
        let roots = [
            (HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths"),
            (HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\App Paths"),
            (HKEY_CURRENT_USER, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths"),
        ];
        for (hive, base) in roots {
            let root = RegKey::predef(hive);
            let subkey = match root.open_subkey_with_flags(format!("{}\\{}", base, exe_name), KEY_READ) {
                Ok(key) => key,
                Err(_) => continue,
            };
            let value: String = match subkey.get_value("") {
                Ok(value) => value,
                Err(_) => continue,
            };
            let path = PathBuf::from(value.trim_matches('"'));
            if path.is_file() {
                return Some(path);
            }
        }
        None
    }

    fn which_in_path(&self, name: &str) -> Option<PathBuf> {
        let path_env = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_env) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    fn registry_powershell_core_install_dirs(&self) -> Vec<PathBuf> {
        let roots = [
            (HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\PowerShellCore\\InstalledVersions"),
            (HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Microsoft\\PowerShellCore\\InstalledVersions"),
            (HKEY_CURRENT_USER, "SOFTWARE\\Microsoft\\PowerShellCore\\InstalledVersions"),
        ];
        let mut result: Vec<PathBuf> = Vec::new();
        for (hive, base) in roots {
            let root = RegKey::predef(hive);
            let parent = match root.open_subkey_with_flags(base, KEY_READ) {
                Ok(key) => key,
                Err(_) => continue,
            };
            for name in parent.enum_keys().flatten() {
                let version_key = match parent.open_subkey_with_flags(&name, KEY_READ) {
                    Ok(key) => key,
                    Err(_) => continue,
                };
                let install_location: String = match version_key.get_value("InstallLocation") {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let path = PathBuf::from(install_location);
                if path.is_dir() && !result.iter().any(|existing| existing == &path) {
                    result.push(path);
                }
            }
        }
        result
    }

    fn bash_from_git_exe(&self, git_exe: &Path) -> Option<PathBuf> {
        let parent = git_exe.parent()?;
        let candidates = [parent.join("..").join("bin").join("bash.exe"), parent.join("bash.exe")];
        for candidate in candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    fn registry_git_install_path(&self) -> Option<PathBuf> {
        let roots = [(HKEY_LOCAL_MACHINE, "SOFTWARE\\GitForWindows"), (HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\GitForWindows"), (HKEY_CURRENT_USER, "SOFTWARE\\GitForWindows")];
        for (hive, path) in roots {
            let root = RegKey::predef(hive);
            let key = match root.open_subkey_with_flags(path, KEY_READ) {
                Ok(key) => key,
                Err(_) => continue,
            };
            let install_path: String = match key.get_value("InstallPath") {
                Ok(value) => value,
                Err(_) => continue,
            };
            let dir = PathBuf::from(install_path);
            if dir.is_dir() {
                return Some(dir);
            }
        }
        None
    }

    fn bash_under_git_install(&self, install_dir: &Path) -> Option<PathBuf> {
        let bash = install_dir.join("bin").join("bash.exe");
        if bash.is_file() {
            return Some(bash);
        }
        None
    }
}

impl ShellDiscover for ShellDiscoverWindows {
    fn discover(&self) -> Vec<ShellDescr> {
        let mut list = self.discover_builtin();
        list.extend(self.discover_optional());
        list
    }

    fn build_exec_args(&self, descr: &ShellDescr, command: &str) -> anyhow::Result<Vec<String>> {
        match descr.shell_id.as_str() {
            "windows.cmd" => Ok(vec!["/C".to_owned(), format!("chcp 65001>nul & {}", command)]),
            "windows.powershell" | "windows.pwsh" => {
                let wrapped = format!("[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); $OutputEncoding = [Console]::OutputEncoding; {}", command);
                Ok(vec!["-NoLogo".to_owned(), "-NonInteractive".to_owned(), "-Command".to_owned(), wrapped])
            }
            "windows.wsl" => Ok(vec!["--".to_owned(), "bash".to_owned(), "-lc".to_owned(), command.to_owned()]),
            "windows.git_bash" => Ok(vec!["-lc".to_owned(), command.to_owned()]),
            other => Err(unknown_exec_shell(other)),
        }
    }
}
