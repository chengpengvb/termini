use std::process::Stdio;
use std::time::Duration;
use summer::tracing::info;
use tokio::process::Command;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptRuntime {
    Deno,
    Node,
    Python,
    Js,
}

impl ScriptRuntime {
    pub fn as_str(self) -> &'static str {
        match self {
            ScriptRuntime::Deno => "deno",
            ScriptRuntime::Node => "node",
            ScriptRuntime::Python => "python",
            ScriptRuntime::Js => "js",
        }
    }

    pub fn script_extension(self) -> &'static str {
        match self {
            ScriptRuntime::Deno => "ts",
            ScriptRuntime::Node => "mjs",
            ScriptRuntime::Python => "py",
            ScriptRuntime::Js => "js",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeProbe {
    pub deno: Option<PathBuf>,
    pub node: Option<PathBuf>,
    pub npm: Option<PathBuf>,
    pub python: Option<PathBuf>,
}

impl RuntimeProbe {
    pub async fn detect() -> RuntimeProbe {
        let deno = Self::verified(Self::find(&["deno.exe"], &["deno"]), &["--version"], None).await;
        let node = Self::verified(Self::find(&["node.exe"], &["node"]), &["--version"], None).await;
        let npm = match node {
            Some(_) => Self::verified(Self::find(&["npm.cmd"], &["npm"]), &["--version"], None).await,
            None => None,
        };
        let python = Self::verified(Self::find(&["python3.exe", "python.exe"], &["python3", "python"]), &["--version"], Some("Python 3")).await;
        let probe = RuntimeProbe { deno, node, npm, python };
        info!(
            "[ai] 本机运行时探测 deno={} node={} npm={} python={}",
            probe.deno.as_deref().map(Path::display).map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
            probe.node.as_deref().map(Path::display).map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
            probe.npm.as_deref().map(Path::display).map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
            probe.python.as_deref().map(Path::display).map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
        );
        probe
    }

    pub fn has_node(&self) -> bool {
        self.node.is_some() && self.npm.is_some()
    }

    pub fn has_python(&self) -> bool {
        self.python.is_some()
    }

    pub fn available(&self, managed_deno: bool) -> Vec<ScriptRuntime> {
        let mut list = Vec::new();
        if self.deno.is_some() || managed_deno {
            list.push(ScriptRuntime::Deno);
        }
        if self.has_node() {
            list.push(ScriptRuntime::Node);
        }
        if self.has_python() {
            list.push(ScriptRuntime::Python);
        }
        list.push(ScriptRuntime::Js);
        list
    }

    pub fn preferred(&self, managed_deno: bool) -> ScriptRuntime {
        self.available(managed_deno).first().copied().unwrap_or(ScriptRuntime::Js)
    }

    fn find(windows_names: &[&str], unix_names: &[&str]) -> Option<PathBuf> {
        let names = if cfg!(windows) { windows_names } else { unix_names };
        let path = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    async fn verified(candidate: Option<PathBuf>, args: &[&str], expect_prefix: Option<&str>) -> Option<PathBuf> {
        let path = candidate?;
        let mut command = Command::new(&path);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        let output = tokio::time::timeout(PROBE_TIMEOUT, command.output()).await.ok()?.ok()?;
        if !output.status.success() {
            return None;
        }
        if let Some(prefix) = expect_prefix {
            let text = String::from_utf8_lossy(&output.stdout);
            if !text.trim_start().starts_with(prefix) {
                return None;
            }
        }
        Some(path)
    }
}
