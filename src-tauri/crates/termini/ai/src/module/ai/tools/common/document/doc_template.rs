use std::io::Write;
use std::io::Cursor;
use summer::tracing::info;
use tokio::process::Command;
use std::path::{Path, PathBuf};
use crate::util::hash_util::sha256_hex;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::document::external_tool::PANDOC_VERSION;
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;

const TYPST_TEMPLATE: &str = include_str!("../../../../../../resource/document/template.typst");
const REFERENCE_STYLES: &str = include_str!("../../../../../../resource/document/reference-styles.xml");
pub const OUTPUT_SPEC: &str = include_str!("../../../../../../resource/document/output-spec.md");

const TEMPLATE_DIR: &str = "tools/document";
const STYLES_ENTRY: &str = "word/styles.xml";

pub struct DocxFonts {
    pub latin: &'static str,
    pub cjk: &'static str,
    pub mono: &'static str,
}

pub fn docx_fonts() -> DocxFonts {
    if cfg!(target_os = "windows") {
        DocxFonts { latin: "Calibri", cjk: "Microsoft YaHei", mono: "Consolas" }
    } else if cfg!(target_os = "macos") {
        DocxFonts { latin: "Helvetica Neue", cjk: "PingFang SC", mono: "Menlo" }
    } else {
        DocxFonts { latin: "Liberation Sans", cjk: "Noto Sans CJK SC", mono: "DejaVu Sans Mono" }
    }
}

fn template_dir() -> Result<PathBuf, AiToolError> {
    let dir = ToolInstaller::data_dir()?.join(TEMPLATE_DIR);
    std::fs::create_dir_all(&dir).map_err(|error| AiToolError::Execution(format!("create {}: {}", dir.display(), error)))?;
    Ok(dir)
}

fn short_hash(text: &str) -> String {
    sha256_hex(text).chars().take(8).collect()
}

pub fn typst_template_path() -> Result<PathBuf, AiToolError> {
    let path = template_dir()?.join(format!("template-{}.typst", short_hash(TYPST_TEMPLATE)));
    if !path.exists() {
        std::fs::write(&path, TYPST_TEMPLATE).map_err(|error| AiToolError::Execution(format!("write {}: {}", path.display(), error)))?;
        info!("[ai] 已写出 typst 模板 {}", path.display());
    }
    Ok(path)
}

fn rendered_styles() -> String {
    let fonts = docx_fonts();
    REFERENCE_STYLES.replace("{{LATIN_FONT}}", fonts.latin).replace("{{CJK_FONT}}", fonts.cjk).replace("{{MONO_FONT}}", fonts.mono)
}

// pandoc 的默认 reference.docx 只换掉样式表, 其余部件(主题/编号/设置)原样保留
pub async fn reference_docx_path(pandoc: &Path) -> Result<PathBuf, AiToolError> {
    let styles = rendered_styles();
    let path = template_dir()?.join(format!("reference-{}-{}.docx", PANDOC_VERSION, short_hash(&styles)));
    if path.exists() {
        return Ok(path);
    }
    let default_docx = print_default_reference(pandoc).await?;
    let rebuilt = replace_styles(&default_docx, &styles)?;
    std::fs::write(&path, rebuilt).map_err(|error| AiToolError::Execution(format!("write {}: {}", path.display(), error)))?;
    info!("[ai] 已生成 Word 样式模板 {}", path.display());
    Ok(path)
}

async fn print_default_reference(pandoc: &Path) -> Result<Vec<u8>, AiToolError> {
    let mut command = Command::new(pandoc);
    command.arg("--print-default-data-file").arg("reference.docx").kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let output = command.output().await.map_err(|error| AiToolError::Execution(format!("failed to start pandoc: {}", error)))?;
    if !output.status.success() {
        return Err(AiToolError::Execution(format!("pandoc --print-default-data-file exit code {}: {}", output.status.code().unwrap_or(-1), String::from_utf8_lossy(&output.stderr).trim())));
    }
    Ok(output.stdout)
}

fn replace_styles(docx: &[u8], styles: &str) -> Result<Vec<u8>, AiToolError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(docx)).map_err(|error| AiToolError::Execution(format!("read default reference.docx: {}", error)))?;
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| AiToolError::Execution(format!("read reference.docx entry: {}", error)))?;
        if entry.name() == STYLES_ENTRY {
            out.start_file(STYLES_ENTRY, options).map_err(|error| AiToolError::Execution(error.to_string()))?;
            out.write_all(styles.as_bytes()).map_err(|error| AiToolError::Execution(error.to_string()))?;
            continue;
        }
        out.raw_copy_file(entry).map_err(|error| AiToolError::Execution(format!("copy reference.docx entry: {}", error)))?;
    }
    let cursor = out.finish().map_err(|error| AiToolError::Execution(error.to_string()))?;
    Ok(cursor.into_inner())
}
