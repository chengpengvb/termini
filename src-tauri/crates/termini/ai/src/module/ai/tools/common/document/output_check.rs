use std::path::Path;
use crate::util::id_factory::uuid;
use crate::module::ai::tools::common::pdf::pdfium_text;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::document::pandoc::PandocConverter;

const PDF_PROBE_PAGES: usize = 600;

pub struct DocCheck {
    pub summary: String,
    pub problems: Vec<String>,
}

impl DocCheck {
    pub fn describe(&self) -> String {
        let mut text = format!("[回读] {}", self.summary);
        for problem in &self.problems {
            text.push_str("\n[回读] 异常: ");
            text.push_str(problem);
        }
        text
    }
}

pub async fn check_docx(converter: &PandocConverter, path: &Path) -> Result<DocCheck, AiToolError> {
    let media_dir = std::env::temp_dir().join(format!("termini-docx-check-{}", uuid()));
    let result = converter.docx_to_markdown(path, &media_dir).await;
    let _ = tokio::fs::remove_dir_all(&media_dir).await;
    let markdown = result?;
    Ok(inspect_markdown(&markdown))
}

pub fn check_pdf(pdfium_dll: &Path, path: &Path) -> Result<DocCheck, AiToolError> {
    let extract = pdfium_text::extract_text_via_pdfium(pdfium_dll, path, 0, PDF_PROBE_PAGES)?;
    let mut problems = Vec::new();
    if extract.pages == 0 {
        problems.push("PDF 没有任何页面".to_string());
    } else if extract.chars == 0 {
        problems.push("PDF 文字层为空, 内容可能没有渲染出来".to_string());
    }
    Ok(DocCheck { summary: format!("共 {} 页, 文字层 {} 字", extract.pages, extract.chars), problems })
}

fn inspect_markdown(markdown: &str) -> DocCheck {
    let mut headings = 0usize;
    let mut tables = 0usize;
    let mut empty_tables = 0usize;
    let mut images = 0usize;
    let mut chars = 0usize;
    let mut in_table = false;
    let mut table_rows = 0usize;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            headings += 1;
        }
        images += trimmed.matches("![").count();
        chars += trimmed.chars().filter(|ch| !ch.is_whitespace()).count();
        if trimmed.starts_with('|') {
            if !in_table {
                in_table = true;
                tables += 1;
                table_rows = 0;
            } else if !is_separator(trimmed) {
                table_rows += 1;
            }
            continue;
        }
        if in_table {
            in_table = false;
            if table_rows == 0 {
                empty_tables += 1;
            }
        }
    }
    if in_table && table_rows == 0 {
        empty_tables += 1;
    }

    let mut problems = Vec::new();
    if chars == 0 {
        problems.push("正文为空, 文件里没有可读文字".to_string());
    }
    if empty_tables > 0 {
        problems.push(format!("{} 个表格只有表头没有数据行", empty_tables));
    }
    DocCheck { summary: format!("{} 个标题, {} 个表格, {} 张图, 正文 {} 字", headings, tables, images, chars), problems }
}

fn is_separator(line: &str) -> bool {
    let body = line.trim_matches('|');
    !body.is_empty() && body.chars().all(|ch| matches!(ch, '-' | ':' | '|' | ' '))
}
