use std::fs;
use std::path::PathBuf;
use summer::tracing::warn;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::util::time_util::now_millis;
use crate::module::ai::shared::chat_item_format::truncate_chars;
use crate::module::ai::derived::derived_store::{Derived, DerivedKind, DerivedStore};

const MAX_ENTRIES: usize = 40;
const QUESTION_MAX_CHARS: usize = 40;
const MATERIALS_DIR: &str = "_materials";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub lines: usize,
    pub gist: String,
    pub source: String,
    pub created_ms: i64,
    pub kind: DerivedKind,
    pub derived_path: String,
    pub question: Option<String>,
}

impl DerivedStore {
    fn materials_file(chat_id: &str) -> Result<PathBuf> {
        Ok(Self::root()?.join(MATERIALS_DIR).join(format!("{}.json", chat_id)))
    }

    pub fn materials(&self, chat_id: &str) -> Vec<Material> {
        let Ok(path) = Self::materials_file(chat_id) else {
            return Vec::new();
        };
        let Ok(text) = fs::read_to_string(&path) else {
            return Vec::new();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    /// 同一份派生文档只记一条(重读/刷新后覆盖), 超出上限丢最早的
    pub fn record_material(&self, chat_id: &str, derived: &Derived) {
        let material = Material {
            kind: derived.meta.kind,
            source: derived.meta.source.clone(),
            derived_path: derived.path.to_string_lossy().to_string(),
            question: derived.meta.question.clone(),
            lines: derived.meta.lines,
            gist: derived.meta.gist.clone(),
            created_ms: now_millis(),
        };
        if let Err(error) = self.append_material(chat_id, material) {
            warn!("[ai] 记录材料清单失败 chat_id={} error={:#}", chat_id, error);
        }
    }

    fn append_material(&self, chat_id: &str, material: Material) -> Result<()> {
        let path = Self::materials_file(chat_id)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let mut list = self.materials(chat_id);
        list.retain(|item| item.derived_path != material.derived_path);
        list.push(material);
        if list.len() > MAX_ENTRIES {
            let drop = list.len() - MAX_ENTRIES;
            list.drain(..drop);
        }
        fs::write(&path, serde_json::to_string_pretty(&list)?).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    pub fn render_materials(&self, chat_id: &str) -> Option<String> {
        let list = self.materials(chat_id);
        if list.is_empty() {
            return None;
        }
        let root = Self::root().ok()?;
        let mut out = String::from("[材料清单]\n本对话已读取并保存为全文的材料, 折叠后仍可按路径取回; 需要内容时用 file_read 按 offset/limit 翻页, 找内容用 code_search 或 code_semantic (root=");
        out.push_str(&root.to_string_lossy());
        out.push_str("); 图片识别结果按提示词分别保存, 已有能用的就直接读, 不要重复识别:\n");
        for (index, item) in list.iter().enumerate() {
            let question = item.question.as_deref().map(|text| format!(" 「{}」", truncate_chars(text, QUESTION_MAX_CHARS))).unwrap_or_default();
            out.push_str(&format!("{}. [{}]{} {} → {} ({} 行) {}\n", index + 1, item.kind.label(), question, item.source, item.derived_path, item.lines, item.gist));
        }
        Some(out)
    }
}
