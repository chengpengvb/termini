use std::io::Cursor;
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use crate::module::ai::tools::common::image::transcript;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::image::vision_transcriber::TranscribeUnit;
use crate::module::ai::tools::common::image::image_prepare::{self, PrepareOptions, PreparedImage};

const RENDER_WIDTH_PX: i32 = 1600;
const PAGE_MARK_SUFFIX: &str = " ---";
const PAGE_MARK_PREFIX: &str = "--- page ";
pub const FULL_EXTRACT_MAX_PAGES: usize = 600;
const SCANNED_TEXT_CHARS_PER_PAGE: usize = 20;

static PDFIUM_INSTANCE: OnceLock<Result<Mutex<Pdfium>, String>> = OnceLock::new();

pub struct PdfPageText {
    pub text: String,
    pub taken: usize,
    pub total: usize,
}

pub struct PdfExtract {
    pub text: String,
    pub chars: usize,
    pub pages: usize,
}

impl PdfExtract {
    pub fn looks_scanned(&self) -> bool {
        self.pages > 0 && self.chars < self.pages.min(FULL_EXTRACT_MAX_PAGES) * SCANNED_TEXT_CHARS_PER_PAGE
    }
}

pub fn resolve_pdf_path(path: &str) -> Result<PathBuf, AiToolError> {
    if path.trim().is_empty() {
        return Err(AiToolError::Execution("path is empty".to_string()));
    }
    let raw = PathBuf::from(path);
    if !raw.is_absolute() {
        return Err(AiToolError::Execution(format!("path must be absolute: {}", path)));
    }
    if !raw.exists() {
        return Err(AiToolError::Execution(format!("file not found: {}", raw.display())));
    }
    if !raw.is_file() {
        return Err(AiToolError::Execution(format!("not a file: {}", raw.display())));
    }
    if !raw.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf")) {
        return Err(AiToolError::Execution(format!("not a pdf file: {}", raw.display())));
    }
    Ok(raw)
}

fn pdfium_instance(dll_path: &Path) -> Result<MutexGuard<'static, Pdfium>, AiToolError> {
    let stored = PDFIUM_INSTANCE.get_or_init(|| Pdfium::bind_to_library(dll_path).map(Pdfium::new).map(Mutex::new).map_err(|error| error.to_string()));
    let guarded = stored.as_ref().map_err(|error| AiToolError::Execution(format!("bind pdfium: {}", error)))?;
    guarded.lock().map_err(|error| AiToolError::Execution(format!("pdfium lock poisoned: {}", error)))
}

pub fn page_mark(page_number: usize) -> String {
    format!("{}{}{}", PAGE_MARK_PREFIX, page_number, PAGE_MARK_SUFFIX)
}

pub fn extract_text_via_pdfium(dll_path: &Path, pdf_path: &Path, start_page: usize, page_limit: usize) -> Result<PdfExtract, AiToolError> {
    let pdfium = pdfium_instance(dll_path)?;
    let document = pdfium.load_pdf_from_file(pdf_path, None).map_err(|error| AiToolError::Execution(format!("load pdf: {}", error)))?;
    let total = document.pages().len() as usize;
    if start_page >= total {
        return Ok(PdfExtract { text: String::new(), chars: 0, pages: total });
    }
    let end_page = (start_page + page_limit).min(total);
    let mut text = String::new();
    let mut chars = 0usize;
    for index in start_page..end_page {
        let page = document.pages().get(index as i32).map_err(|error| AiToolError::Execution(format!("page {}: {}", index + 1, error)))?;
        let page_text = page.text().map_err(|error| AiToolError::Execution(format!("page text: {}", error)))?.all().replace("\r\n", "\n").replace('\r', "\n");
        chars += page_text.chars().filter(|ch| !ch.is_whitespace()).count();
        text.push_str(&transcript::fenced_page(&page_mark(index + 1), &page_text));
        text.push('\n');
    }
    Ok(PdfExtract { text, chars, pages: total })
}

pub fn render_pages_via_pdfium(dll_path: &Path, pdf_path: &Path, max_pages: usize, options: PrepareOptions) -> Result<(Vec<TranscribeUnit>, usize), AiToolError> {
    let pdfium = pdfium_instance(dll_path)?;
    let document = pdfium.load_pdf_from_file(pdf_path, None).map_err(|error| AiToolError::Execution(format!("load pdf: {}", error)))?;
    let total = document.pages().len() as usize;
    let count = total.min(max_pages);
    let config = PdfRenderConfig::new().set_target_width(RENDER_WIDTH_PX);
    let mut units = Vec::with_capacity(count);
    for index in 0..count {
        let page = document.pages().get(index as i32).map_err(|error| AiToolError::Execution(format!("page {}: {}", index + 1, error)))?;
        let bitmap = page.render_with_config(&config).map_err(|error| AiToolError::Execution(format!("render page {}: {}", index + 1, error)))?;
        let image = bitmap.as_image().map_err(|error| AiToolError::Execution(format!("page {} bitmap: {}", index + 1, error)))?;
        let mut png = Vec::new();
        image.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).map_err(|error| AiToolError::Execution(format!("page {} png: {}", index + 1, error)))?;
        let prepared = image_prepare::prepare(png, "image/png", options);
        let images: Vec<PreparedImage> = prepared.images;
        units.push(TranscribeUnit { label: Some(page_mark(index + 1)), images });
    }
    Ok((units, total))
}

pub fn window_from_full(full: &str, start_page: usize, page_limit: usize) -> PdfPageText {
    let mut pages: Vec<String> = Vec::new();
    for line in full.lines() {
        if line.starts_with(PAGE_MARK_PREFIX) && line.ends_with(PAGE_MARK_SUFFIX) {
            pages.push(format!("{}\n", line));
            continue;
        }
        if let Some(last) = pages.last_mut() {
            last.push_str(line);
            last.push('\n');
        }
    }
    let total = pages.len();
    if start_page >= total {
        return PdfPageText { text: String::new(), taken: 0, total };
    }
    let end_page = (start_page + page_limit).min(total);
    PdfPageText { text: pages[start_page..end_page].concat(), taken: end_page - start_page, total }
}
