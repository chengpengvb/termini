use anyhow::Result;
use std::sync::OnceLock;
use std::time::Duration;
use summer::plugin::Service;
use tokio::process::Command;
use async_trait::async_trait;
use termini_macros::base_app;
use std::path::{Path, PathBuf};
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::base::base_app::{AppContext, BaseApp};
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::document::doc_template;
use crate::module::ai::tools::common::document::external_tool::ExternalTool;
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;

const MARKDOWN_OUTPUT_FORMAT: &str = "gfm";
const CONVERT_TIMEOUT: Duration = Duration::from_secs(180);
const FONT_LIST_TIMEOUT: Duration = Duration::from_secs(30);
const MARKDOWN_FORMAT: &str = "gfm+hard_line_breaks+raw_attribute";
const MACOS_FONTS: &[&str] = &["PingFang SC", "Hiragino Sans GB", "Heiti SC", "Songti SC", "Arial Unicode MS"];
const WINDOWS_FONTS: &[&str] = &["Microsoft YaHei", "DengXian", "SimHei", "Noto Sans SC", "SimSun", "Malgun Gothic", "Yu Gothic"];
const LINUX_FONTS: &[&str] = &["Noto Sans CJK SC", "Source Han Sans SC", "WenQuanYi Micro Hei", "WenQuanYi Zen Hei", "Droid Sans Fallback", "DejaVu Sans"];
const MACOS_MONO_FONTS: &[&str] = &["Menlo", "SF Mono", "Monaco"];
const WINDOWS_MONO_FONTS: &[&str] = &["Cascadia Mono", "Consolas", "Courier New"];
const LINUX_MONO_FONTS: &[&str] = &["DejaVu Sans Mono", "Noto Sans Mono", "Liberation Mono"];

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

static TYPST_FONTS: OnceLock<Vec<String>> = OnceLock::new();

pub struct ConvertReport {
    pub bytes: u64,
    pub output: PathBuf,
    pub warnings: String,
    pub font: Option<String>,
}

#[base_app(sort = 765)]
#[derive(Clone, Service)]
pub struct PandocConverter {
    #[inject(component)]
    tool_installer: ToolInstaller,
}

#[async_trait]
impl BaseApp for PandocConverter {
    fn name(&self) -> &'static str {
        "PandocConverter"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Ok(())
    }
}

impl PandocConverter {
    pub async fn markdown_to_docx(&self, markdown: &str, output: &Path) -> Result<ConvertReport, AiToolError> {
        let pandoc = self.tool_installer.ensure(ExternalTool::Pandoc).await?;
        let reference = doc_template::reference_docx_path(&pandoc).await?;
        let input = Self::write_input(markdown).await?;
        let mut command = Self::command(&pandoc);
        command.arg(&input).arg(format!("--from={}", MARKDOWN_FORMAT)).arg("--to=docx").arg("--reference-doc").arg(&reference).arg("--output").arg(output);
        let result = Self::run(command, "pandoc").await;
        let _ = tokio::fs::remove_file(&input).await;
        let warnings = result?;
        Self::report(output, Some(doc_template::docx_fonts().cjk.to_string()), warnings).await
    }

    pub async fn markdown_to_pdf(&self, markdown: &str, output: &Path) -> Result<ConvertReport, AiToolError> {
        let pandoc = self.tool_installer.ensure(ExternalTool::Pandoc).await?;
        let typst = self.tool_installer.ensure(ExternalTool::Typst).await?;
        let template = doc_template::typst_template_path()?;
        let families = Self::typst_fonts(&typst).await;
        let main_fonts = Self::pick_fonts(&families, Self::main_candidates());
        let mut code_fonts = Self::pick_fonts(&families, Self::mono_candidates());
        code_fonts.extend(main_fonts.iter().cloned());
        if main_fonts.is_empty() {
            warn!("[ai] 本机没有找到候选中日韩字体, PDF 将使用 typst 默认字体 candidates={:?}", Self::main_candidates());
        }
        let input = Self::write_input(markdown).await?;
        let mut command = Self::command(&pandoc);
        command.arg(&input).arg(format!("--from={}", MARKDOWN_FORMAT)).arg("--output").arg(output).arg("--pdf-engine").arg(&typst).arg("--template").arg(&template);
        for name in &main_fonts {
            command.arg("-V").arg(format!("mainfont={}", name));
        }
        for name in &code_fonts {
            command.arg("-V").arg(format!("codefont={}", name));
        }
        let result = Self::run(command, "pandoc").await;
        let _ = tokio::fs::remove_file(&input).await;
        let warnings = result?;
        Self::report(output, main_fonts.first().cloned(), warnings).await
    }

    pub async fn docx_to_markdown(&self, source: &Path, media_dir: &Path) -> Result<String, AiToolError> {
        let pandoc = self.tool_installer.ensure(ExternalTool::Pandoc).await?;
        tokio::fs::create_dir_all(media_dir).await.map_err(|error| AiToolError::Execution(format!("create {}: {}", media_dir.display(), error)))?;
        let mut command = Self::command(&pandoc);
        command.current_dir(media_dir).arg(source).arg("--from=docx").arg(format!("--to={}", MARKDOWN_OUTPUT_FORMAT)).arg("--wrap=none").arg("--extract-media=.");
        let output = tokio::time::timeout(CONVERT_TIMEOUT, command.output())
            .await
            .map_err(|_| AiToolError::Execution(format!("pandoc did not finish within {}s", CONVERT_TIMEOUT.as_secs())))?
            .map_err(|error| AiToolError::Execution(format!("failed to start pandoc: {}", error)))?;
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            return Err(AiToolError::Execution(format!("pandoc exit code {}: {}", output.status.code().unwrap_or(-1), stderr)));
        }
        if !stderr.is_empty() {
            warn!("[ai] docx 转 markdown 有警告: {}", stderr);
        }
        Ok(String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"))
    }

    async fn report(output: &Path, font: Option<String>, warnings: String) -> Result<ConvertReport, AiToolError> {
        let meta = tokio::fs::metadata(output).await.map_err(|error| AiToolError::Execution(format!("conversion did not produce {}: {}", output.display(), error)))?;
        Ok(ConvertReport { output: output.to_path_buf(), bytes: meta.len(), font, warnings })
    }

    async fn write_input(markdown: &str) -> Result<PathBuf, AiToolError> {
        let path = std::env::temp_dir().join(format!("termini-pandoc-{}.md", uuid()));
        tokio::fs::write(&path, markdown).await.map_err(|error| AiToolError::Execution(format!("failed to write temp file {}: {}", path.display(), error)))?;
        Ok(path)
    }

    fn command(program: &Path) -> Command {
        let mut command = Command::new(program);
        command.kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }

    async fn run(mut command: Command, label: &str) -> Result<String, AiToolError> {
        let output = tokio::time::timeout(CONVERT_TIMEOUT, command.output())
            .await
            .map_err(|_| AiToolError::Execution(format!("{} did not finish within {}s", label, CONVERT_TIMEOUT.as_secs())))?
            .map_err(|error| AiToolError::Execution(format!("failed to start {}: {}", label, error)))?;
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            return Err(AiToolError::Execution(format!("{} exit code {}: {}", label, output.status.code().unwrap_or(-1), stderr)));
        }
        if !stderr.is_empty() {
            warn!("[ai] {} 有警告: {}", label, stderr);
        }
        Ok(stderr)
    }

    fn main_candidates() -> &'static [&'static str] {
        if cfg!(target_os = "windows") {
            WINDOWS_FONTS
        } else if cfg!(target_os = "macos") {
            MACOS_FONTS
        } else {
            LINUX_FONTS
        }
    }

    fn mono_candidates() -> &'static [&'static str] {
        if cfg!(target_os = "windows") {
            WINDOWS_MONO_FONTS
        } else if cfg!(target_os = "macos") {
            MACOS_MONO_FONTS
        } else {
            LINUX_MONO_FONTS
        }
    }

    fn pick_fonts(families: &[String], candidates: &[&str]) -> Vec<String> {
        candidates.iter().filter(|candidate| families.iter().any(|family| family.eq_ignore_ascii_case(candidate))).map(|name| name.to_string()).collect()
    }

    async fn typst_fonts(typst: &Path) -> Vec<String> {
        if let Some(cached) = TYPST_FONTS.get() {
            return cached.clone();
        }
        let mut command = Self::command(typst);
        command.arg("fonts");
        let families = match tokio::time::timeout(FONT_LIST_TIMEOUT, command.output()).await {
            Ok(Ok(output)) if output.status.success() => String::from_utf8_lossy(&output.stdout).lines().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect::<Vec<String>>(),
            Ok(Ok(output)) => {
                warn!("[ai] typst fonts 退出码 {}: {}", output.status.code().unwrap_or(-1), String::from_utf8_lossy(&output.stderr).trim());
                Vec::new()
            }
            Ok(Err(error)) => {
                warn!("[ai] 启动 typst fonts 失败: {}", error);
                Vec::new()
            }
            Err(_) => {
                warn!("[ai] typst fonts 超时");
                Vec::new()
            }
        };
        info!("[ai] typst 可用字体 {} 个", families.len());
        TYPST_FONTS.get_or_init(|| families).clone()
    }
}
