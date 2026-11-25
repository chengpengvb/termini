use std::fs;
use sha2::Digest;
use sha2::Sha256;
use summer::plugin::Service;
use termini_macros::base_app;
use std::path::{Path, PathBuf};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Context, Result};
use crate::util::time_util::now_millis;
use crate::util::token_util::estimate_text_tokens;
use crate::module::ai::shared::chat_item_format::truncate_chars;

const ID_CHARS: usize = 16;
const META_VERSION: u32 = 2;
const HASH_CHARS: usize = 16;
const VARIANT_CHARS: usize = 12;
const GIST_MAX_CHARS: usize = 80;
pub const WEB_TTL_SECS: u64 = 3600;

const META_SUFFIX: &str = ".meta.json";
const DEFAULT_VARIANT: &str = "content";
pub const DERIVED_DIR: &str = "derived";
const DATA_DIR_ENV: &str = "TERMINI_DATA_DIR";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivedKind {
    Image,
    Pdf,
    Docx,
    Xlsx,
    Web,
}

impl DerivedKind {
    pub fn label(self) -> &'static str {
        match self {
            DerivedKind::Image => "图片识别",
            DerivedKind::Pdf => "PDF 文本",
            DerivedKind::Docx => "Word 文本",
            DerivedKind::Xlsx => "Excel 文本",
            DerivedKind::Web => "网页快照",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedMeta {
    pub gist: String,
    pub version: u32,
    pub lines: usize,
    pub tokens: usize,
    pub source: String,
    pub created_ms: i64,
    pub kind: DerivedKind,
    pub ttl_secs: Option<u64>,
    pub model: Option<String>,
    pub source_size: Option<u64>,
    pub question: Option<String>,
    pub source_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Derived {
    pub cached: bool,
    pub dir: PathBuf,
    pub path: PathBuf,
    pub meta: DerivedMeta,
}

impl Derived {
    pub fn header(&self, derived_root: &Path) -> String {
        let mut line = format!("[全文已保存 · {}] {}\n来源: {} | {} 行 | 约 {} tokens", self.meta.kind.label(), self.path.display(), self.meta.source, self.meta.lines, self.meta.tokens);
        if let Some(question) = &self.meta.question {
            line.push_str(&format!(" | 提示词: {}", truncate_chars(question, GIST_MAX_CHARS)));
        }
        if let Some(model) = &self.meta.model {
            line.push_str(&format!(" | 模型: {}", model));
        }
        if self.cached {
            line.push_str(" | 命中缓存");
        }
        line.push_str(&format!("\n读全文用 file_read 该路径按 offset/limit 翻页; 找内容用 code_search 或 code_semantic, root={}", derived_root.display()));
        line
    }
}

#[base_app(sort = 758)]
#[derive(Clone, Service)]
pub struct DerivedStore {}

impl BaseApp for DerivedStore {
    fn name(&self) -> &'static str {
        "DerivedStore"
    }
}

impl DerivedStore {
    pub fn root() -> Result<PathBuf> {
        let data_dir = std::env::var(DATA_DIR_ENV).with_context(|| format!("failed to read {}", DATA_DIR_ENV))?;
        Ok(PathBuf::from(data_dir).join(DERIVED_DIR))
    }

    pub fn content_hash(bytes: &[u8]) -> String {
        hex_digest(bytes, HASH_CHARS)
    }

    pub fn file_key(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/").to_lowercase()
    }

    pub fn web_key(url: &str, raw: bool) -> String {
        if raw {
            format!("{}#raw", url.trim())
        } else {
            url.trim().to_string()
        }
    }

    pub fn variant_of(question: Option<&str>) -> String {
        match question.map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => hex_digest(text.to_lowercase().as_bytes(), VARIANT_CHARS),
            None => DEFAULT_VARIANT.to_string(),
        }
    }

    pub fn dir_for(key: &str) -> Result<PathBuf> {
        Ok(Self::root()?.join(hex_digest(key.as_bytes(), ID_CHARS)))
    }

    pub fn lookup(&self, key: &str, variant: &str, expected_hash: Option<&str>, refresh: bool) -> Option<Derived> {
        if refresh {
            return None;
        }
        let dir = Self::dir_for(key).ok()?;
        let path = dir.join(format!("{}.md", variant));
        if !path.is_file() {
            return None;
        }
        let meta = Self::read_meta(&dir, variant)?;
        if let Some(expected) = expected_hash {
            if meta.source_hash.as_deref() != Some(expected) {
                return None;
            }
        }
        if let Some(ttl) = meta.ttl_secs {
            if now_millis().saturating_sub(meta.created_ms) / 1000 >= ttl as i64 {
                return None;
            }
        }
        Some(Derived { dir, path, meta, cached: true })
    }

    pub fn list_variants(&self, key: &str, expected_hash: Option<&str>) -> Vec<Derived> {
        let Ok(dir) = Self::dir_for(key) else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut out: Vec<Derived> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(variant) = name.strip_suffix(META_SUFFIX) else {
                continue;
            };
            let Some(meta) = Self::read_meta(&dir, variant) else {
                continue;
            };
            if expected_hash.is_some() && meta.source_hash.as_deref() != expected_hash {
                continue;
            }
            let path = dir.join(format!("{}.md", variant));
            if !path.is_file() {
                continue;
            }
            out.push(Derived { dir: dir.clone(), path, meta, cached: true });
        }
        out.sort_by_key(|item| item.meta.created_ms);
        out
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save(
        &self,
        key: &str,
        variant: &str,
        kind: DerivedKind,
        source: &str,
        source_hash: Option<String>,
        source_size: Option<u64>,
        question: Option<String>,
        ttl_secs: Option<u64>,
        model: Option<String>,
        text: &str,
    ) -> Result<Derived> {
        let dir = Self::dir_for(key)?;
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let path = dir.join(format!("{}.md", variant));
        fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
        let meta = DerivedMeta {
            version: META_VERSION,
            kind,
            source: source.to_string(),
            source_hash,
            source_size,
            question: question.map(|text| text.trim().to_string()).filter(|text| !text.is_empty()),
            created_ms: now_millis(),
            ttl_secs,
            model,
            lines: text.lines().count(),
            tokens: estimate_text_tokens(text),
            gist: gist_of(text),
        };
        fs::write(dir.join(format!("{}{}", variant, META_SUFFIX)), serde_json::to_string_pretty(&meta)?).with_context(|| format!("write meta {}", dir.display()))?;
        Ok(Derived { dir, path, meta, cached: false })
    }

    pub fn read_content(derived: &Derived) -> Result<String> {
        fs::read_to_string(&derived.path).map_err(|error| anyhow!("read {}: {}", derived.path.display(), error))
    }

    fn read_meta(dir: &Path, variant: &str) -> Option<DerivedMeta> {
        let meta: DerivedMeta = serde_json::from_str(&fs::read_to_string(dir.join(format!("{}{}", variant, META_SUFFIX))).ok()?).ok()?;
        if meta.version != META_VERSION {
            return None;
        }
        Some(meta)
    }
}

fn hex_digest(bytes: &[u8], chars: usize) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{:02x}", byte)).collect::<String>()[..chars].to_string()
}

pub fn gist_of(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty() && !line.starts_with("<!--") && !line.starts_with("--- page") && !line.starts_with('#')).unwrap_or("");
    truncate_chars(line, GIST_MAX_CHARS)
}
