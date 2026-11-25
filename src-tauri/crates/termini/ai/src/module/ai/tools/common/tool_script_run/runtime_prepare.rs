use std::process::Stdio;
use std::time::Duration;
use summer::tracing::warn;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use std::path::{Path, PathBuf};
use crate::util::id_factory::uuid;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::tool_script_run::ToolScriptRun;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::module::ai::tools::common::document::external_tool::ExternalTool;
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;
use crate::module::ai::tools::common::document::script_runtime::{RuntimeProbe, ScriptRuntime};

const SCRIPT_DIR: &str = "scripts";
const DENO_CACHE_DIR: &str = "tools/deno-cache";
const NODE_DEPS_DIR: &str = "tools/node-deps";
const PYTHON_VENV_DIR: &str = "tools/py-venv";
const INSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const READ_BUFFER: usize = 8 * 1024;
const NODE_PACKAGE_JSON: &str = "{\"name\":\"termini-script-deps\",\"private\":true,\"type\":\"module\"}";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

impl ToolScriptRun {
    pub(super) fn managed_deno_installed() -> bool {
        ToolInstaller::install_path(ExternalTool::Deno).map(|path| path.is_file()).unwrap_or(false)
    }

    pub(super) async fn ensure_probe(&self) -> RuntimeProbe {
        if let Some(probe) = self.probe.read().clone() {
            return probe;
        }
        let detected = RuntimeProbe::detect().await;
        *self.probe.write() = Some(detected.clone());
        detected
    }

    fn base_command(program: &Path) -> Command {
        let mut command = Command::new(program);
        command.env("NO_COLOR", "1").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }

    pub(super) async fn prepare_deno(&self, probe: &RuntimeProbe, script_dir: &Path, code: &str, existing: Option<&Path>) -> Result<(Command, PathBuf), AiToolError> {
        let deno = match probe.deno.clone() {
            Some(path) => path,
            None => self.tool_installer.ensure(ExternalTool::Deno).await?,
        };
        let script = match existing {
            Some(path) => path.to_path_buf(),
            None => Self::write_script(script_dir, code, ScriptRuntime::Deno).await?,
        };
        let mut command = Self::base_command(&deno);
        command
            .arg("run")
            .arg("--allow-all")
            .arg("--quiet")
            .arg("--no-lock")
            .arg("--node-modules-dir=none")
            .env("DENO_DIR", ToolInstaller::data_dir()?.join(DENO_CACHE_DIR))
            .env("DENO_NO_UPDATE_CHECK", "1")
            .arg(&script);
        Ok((command, script))
    }

    pub(super) async fn prepare_node(&self, probe: &RuntimeProbe, code: &str, existing: Option<&Path>, deps: &[String], on_chunk: &ShellChunkCallback) -> Result<(Command, PathBuf), AiToolError> {
        let (Some(node), Some(npm)) = (probe.node.clone(), probe.npm.clone()) else {
            return Err(AiToolError::Execution("no usable node/npm on this machine; use runtime=deno or python".to_string()));
        };
        let deps_dir = ToolInstaller::data_dir()?.join(NODE_DEPS_DIR);
        if let Some(path) = existing.filter(|path| !path.starts_with(&deps_dir)) {
            return Err(AiToolError::Execution(format!(
                "runtime=node 只能重跑本工具生成的脚本(在 {} 下), 别处的脚本解析不到已安装的依赖: {}。首次生成改用 code 参数让工具自己落盘, 需要的包写进 deps, 之后再用返回的 script_path 重跑; 不要自己 npm install",
                deps_dir.join(SCRIPT_DIR).display(),
                path.display()
            )));
        }
        tokio::fs::create_dir_all(&deps_dir).await.map_err(|error| AiToolError::Execution(format!("create {}: {}", deps_dir.display(), error)))?;
        let package_json = deps_dir.join("package.json");
        if !package_json.is_file() {
            tokio::fs::write(&package_json, NODE_PACKAGE_JSON).await.map_err(|error| AiToolError::Execution(format!("failed to write {}: {}", package_json.display(), error)))?;
        }
        let missing = deps.iter().filter(|dep| !deps_dir.join("node_modules").join(Self::package_dir_name(dep)).is_dir()).cloned().collect::<Vec<String>>();
        if !missing.is_empty() {
            on_chunk(format!("[deps] npm install {}\n", missing.join(" ")).into_bytes()).await;
            let mut install = Self::base_command(&npm);
            install.current_dir(&deps_dir).arg("install").arg("--no-audit").arg("--no-fund").arg("--loglevel=error").arg("--no-package-lock").args(&missing);
            Self::run_install(install, "npm install", on_chunk).await?;
        }
        let script = match existing {
            Some(path) => path.to_path_buf(),
            None => Self::write_script(&deps_dir.join(SCRIPT_DIR), code, ScriptRuntime::Node).await?,
        };
        let mut command = Self::base_command(&node);
        command.arg(&script);
        Ok((command, script))
    }

    pub(super) async fn prepare_python(
        &self,
        probe: &RuntimeProbe,
        script_dir: &Path,
        code: &str,
        existing: Option<&Path>,
        deps: &[String],
        on_chunk: &ShellChunkCallback,
    ) -> Result<(Command, PathBuf), AiToolError> {
        let Some(system_python) = probe.python.clone() else {
            return Err(AiToolError::Execution("no usable Python 3 on this machine; use runtime=deno or node".to_string()));
        };
        let venv_dir = ToolInstaller::data_dir()?.join(PYTHON_VENV_DIR);
        let venv_python = Self::venv_python(&venv_dir);
        if !venv_python.is_file() {
            on_chunk("[deps] python -m venv\n".to_string().into_bytes()).await;
            let mut create = Self::base_command(&system_python);
            create.arg("-m").arg("venv").arg(&venv_dir);
            Self::run_install(create, "python -m venv", on_chunk).await?;
        }
        if !deps.is_empty() {
            on_chunk(format!("[deps] pip install {}\n", deps.join(" ")).into_bytes()).await;
            let mut install = Self::base_command(&venv_python);
            install.env("PIP_DISABLE_PIP_VERSION_CHECK", "1").arg("-m").arg("pip").arg("install").arg("--quiet").args(deps);
            Self::run_install(install, "pip install", on_chunk).await?;
        }
        let script = match existing {
            Some(path) => path.to_path_buf(),
            None => Self::write_script(script_dir, code, ScriptRuntime::Python).await?,
        };
        let mut command = Self::base_command(&venv_python);
        command.env("PYTHONIOENCODING", "utf-8").env("PYTHONUNBUFFERED", "1").arg(&script);
        Ok((command, script))
    }

    fn venv_python(venv_dir: &Path) -> PathBuf {
        if cfg!(windows) {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        }
    }

    fn package_dir_name(spec: &str) -> String {
        let trimmed = spec.trim();
        if let Some(rest) = trimmed.strip_prefix('@') {
            return match rest.find('@') {
                Some(index) => format!("@{}", &rest[..index]),
                None => trimmed.to_string(),
            };
        }
        match trimmed.find('@') {
            Some(index) => trimmed[..index].to_string(),
            None => trimmed.to_string(),
        }
    }

    async fn run_install(mut command: Command, label: &str, on_chunk: &ShellChunkCallback) -> Result<(), AiToolError> {
        let mut child = command.spawn().map_err(|error| AiToolError::Execution(format!("failed to start {}: {}", label, error)))?;
        let stdout = child.stdout.take().map(|pipe| tokio::spawn(Self::pump(pipe, on_chunk.clone())));
        let stderr = child.stderr.take().map(|pipe| tokio::spawn(Self::pump(pipe, on_chunk.clone())));
        let status = tokio::time::timeout(INSTALL_TIMEOUT, child.wait()).await;
        for pump in [stdout, stderr].into_iter().flatten() {
            let _ = pump.await;
        }
        match status {
            Ok(Ok(status)) if status.success() => Ok(()),
            Ok(Ok(status)) => Err(AiToolError::Execution(format!("{} exit code {}", label, status.code().unwrap_or(-1)))),
            Ok(Err(error)) => Err(AiToolError::Execution(format!("{} failed: {}", label, error))),
            Err(_) => {
                let _ = child.kill().await;
                Err(AiToolError::Execution(format!("{} did not finish within {}s", label, INSTALL_TIMEOUT.as_secs())))
            }
        }
    }

    pub(super) async fn write_script(dir: &Path, code: &str, runtime: ScriptRuntime) -> Result<PathBuf, AiToolError> {
        tokio::fs::create_dir_all(dir).await.map_err(|error| AiToolError::Execution(format!("create {}: {}", dir.display(), error)))?;
        let path = dir.join(format!("{}.{}", uuid(), runtime.script_extension()));
        tokio::fs::write(&path, code).await.map_err(|error| AiToolError::Execution(format!("failed to write script file {}: {}", path.display(), error)))?;
        Ok(path)
    }

    pub(super) async fn pump<R: AsyncReadExt + Unpin>(mut reader: R, on_chunk: ShellChunkCallback) {
        let mut buffer = vec![0u8; READ_BUFFER];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => return,
                Ok(read) => on_chunk(buffer[..read].to_vec()).await,
                Err(error) => {
                    warn!("[ai] 读取脚本输出失败: {}", error);
                    return;
                }
            }
        }
    }
}
