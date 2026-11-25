use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::document::pandoc::ConvertReport;
use crate::module::ai::tools::common::document::output_check::DocCheck;
use crate::module::ai::tools::common::document::doc_template::OUTPUT_SPEC;

const MAX_SOURCE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MarkdownInput {
    #[serde(default)]
    pub source: Option<String>,

    #[serde(default)]
    pub markdown: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarkdownConfirm {
    pub path: String,
    pub source: Option<String>,
    pub markdown_chars: usize,
}

impl MarkdownInput {
    pub fn load(&self) -> Result<String, AiToolError> {
        let source = self.source.as_deref().map(str::trim).filter(|value| !value.is_empty());
        let inline = self.markdown.as_deref().filter(|value| !value.trim().is_empty());
        match (source, inline) {
            (Some(path), _) => read_source(Path::new(path)),
            (None, Some(text)) => Ok(text.to_string()),
            (None, None) => Err(AiToolError::Execution("at least one of source or markdown is required".to_string())),
        }
    }

    pub fn confirm(&self, path: &str) -> MarkdownConfirm {
        MarkdownConfirm { path: path.to_string(), source: self.source.clone(), markdown_chars: self.markdown.as_deref().map(|text| text.chars().count()).unwrap_or(0) }
    }

    pub fn base_dir(&self) -> Option<PathBuf> {
        let source = self.source.as_deref().map(str::trim).filter(|value| !value.is_empty())?;
        Path::new(source).parent().map(Path::to_path_buf)
    }
}

fn read_source(path: &Path) -> Result<String, AiToolError> {
    if !path.is_absolute() {
        return Err(AiToolError::Execution(format!("source must be absolute: {}", path.display())));
    }
    let meta = std::fs::metadata(path).map_err(|error| AiToolError::Execution(format!("source {}: {}", path.display(), error)))?;
    if !meta.is_file() {
        return Err(AiToolError::Execution(format!("source is not a file: {}", path.display())));
    }
    if meta.len() > MAX_SOURCE_BYTES {
        return Err(AiToolError::Execution(format!("source too large (max {} bytes): {}", MAX_SOURCE_BYTES, path.display())));
    }
    std::fs::read_to_string(path).map_err(|error| AiToolError::Execution(format!("read {}: {}", path.display(), error)))
}

pub fn describe_result(tool: &str, report: &ConvertReport, issues: &[String], check: Option<&DocCheck>) -> String {
    let mut text = format!("[{}] 已写入 {} ({} bytes)", tool, report.output.display(), report.bytes);
    if let Some(font) = report.font.as_deref() {
        text.push_str(&format!(" 字体 {}", font));
    }
    if let Some(check) = check {
        text.push('\n');
        text.push_str(&check.describe());
    }
    if !report.warnings.trim().is_empty() {
        text.push_str(&format!("\n转换警告:\n{}", report.warnings.trim()));
    }
    if !issues.is_empty() {
        text.push_str(&format!("\n[规范提示] 文件已生成, 但 Markdown 有 {} 处不合规, 版式可能受影响; 需要修正时按下面的约定改 Markdown 后重新生成:", issues.len()));
        for issue in issues {
            text.push_str("\n- ");
            text.push_str(issue);
        }
        text.push('\n');
        text.push_str(OUTPUT_SPEC.trim_end());
    }
    text
}

pub fn resolve_output(path: &str, extension: &str) -> Result<PathBuf, AiToolError> {
    if path.trim().is_empty() {
        return Err(AiToolError::Execution("path is empty".to_string()));
    }
    let raw = PathBuf::from(path);
    if !raw.is_absolute() {
        return Err(AiToolError::Execution(format!("path must be absolute: {}", path)));
    }
    if !raw.extension().is_some_and(|ext| ext.eq_ignore_ascii_case(extension)) {
        return Err(AiToolError::Execution(format!("path must end with .{}: {}", extension, path)));
    }
    if let Some(parent) = raw.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| AiToolError::Execution(format!("create {}: {}", parent.display(), error)))?;
        }
    }
    Ok(raw)
}
