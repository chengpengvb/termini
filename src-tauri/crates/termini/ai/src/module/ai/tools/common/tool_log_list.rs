use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use std::path::PathBuf;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::collections::BTreeSet;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::resp::resp_page::QueryPage;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;

const LOG_DIR: &str = "logs";
const TOPN_MAX: usize = 100;
const FILE_SUFFIX: &str = ".log";
const FILE_PREFIX: &str = "termini";

fn default_topn() -> usize {
    10
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolLogListArgs {
    #[serde(default)]
    pub file: Option<String>,

    #[serde(default)]
    pub keyword: Option<String>,

    #[serde(default)]
    pub offset: usize,

    #[serde(default = "default_topn")]
    pub topn: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolLogListOut {
    pub prefixes: Vec<String>,
    pub offset: usize,
    #[serde(flatten)]
    pub page: QueryPage<String>,
}

#[base_app(sort = 782)]
#[derive(Clone, Service)]
pub struct ToolLogList {
    #[inject(component)]
    tool_manager: ToolManager,
}

#[async_trait]
impl BaseApp for ToolLogList {
    fn name(&self) -> &'static str {
        "ToolLogList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolLogList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolLogList
    }

    fn dedup_in_run(&self) -> bool {
        true
    }

    fn read_only(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolLogList {
    fn tokens(keyword: Option<&str>) -> Vec<String> {
        match keyword {
            Some(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    trimmed.split_whitespace().map(|token| token.to_lowercase()).collect()
                }
            }
            None => Vec::new(),
        }
    }

    fn token_hits(line_lower: &str, token: &str) -> bool {
        if line_lower.contains(token) {
            return true;
        }
        if token.starts_with('[') && token.ends_with(']') && token.len() > 2 {
            let inner = &token[1..token.len() - 1];
            return line_lower.contains(inner);
        }
        let wrapped = format!("[{}]", token);
        line_lower.contains(&wrapped)
    }

    fn line_matches(line: &str, tokens: &[String]) -> bool {
        if tokens.is_empty() {
            return true;
        }
        let line_lower = line.to_lowercase();
        tokens.iter().all(|token| Self::token_hits(&line_lower, token))
    }

    fn extract_prefix(line: &str) -> Option<String> {
        let mut start = None;
        for (index, ch) in line.char_indices() {
            match (start, ch) {
                (None, '[') => start = Some(index),
                (Some(from), ']') => {
                    let tag = &line[from..=index];
                    let inner = &tag[1..tag.len() - 1];
                    let valid = !inner.is_empty() && inner.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
                    if valid {
                        let after = index + ']'.len_utf8();
                        if after >= line.len() || line.as_bytes()[after] == b' ' {
                            return Some(tag.to_string());
                        }
                    }
                    start = None;
                }
                (Some(_), ch) if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' => {}
                (Some(_), _) => start = None,
                _ => {}
            }
        }
        None
    }

    fn log_dir() -> Result<PathBuf, AiToolError> {
        Ok(ToolInstaller::data_dir()?.join(LOG_DIR))
    }

    fn list_names() -> Result<Vec<String>, AiToolError> {
        let dir = Self::log_dir()?;
        if !dir.exists() {
            return Ok(Vec::new());
        }
        if !dir.is_dir() {
            return Err(AiToolError::Execution(format!("not a directory: {}", dir.display())));
        }
        let mut names = Vec::new();
        let entries = std::fs::read_dir(&dir).map_err(|error| AiToolError::Execution(format!("{}: {}", dir.display(), error)))?;
        for entry in entries {
            let entry = entry.map_err(|error| AiToolError::Execution(format!("{}: {}", dir.display(), error)))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = match path.file_name() {
                Some(value) => match value.to_str() {
                    Some(text) => text,
                    None => return Err(AiToolError::Execution(format!("log file name is not utf-8: {}", path.display()))),
                },
                None => return Err(AiToolError::Execution(format!("log file name is missing: {}", path.display()))),
            };
            if name.starts_with(FILE_PREFIX) && name.ends_with(FILE_SUFFIX) {
                names.push(name.to_string());
            }
        }
        names.sort();
        names.reverse();
        Ok(names)
    }

    fn resolve_file(name: &str) -> Result<PathBuf, AiToolError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AiToolError::Execution("file is empty".to_string()));
        }
        if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
            return Err(AiToolError::Execution(format!("file is invalid: {}", trimmed)));
        }
        if !trimmed.starts_with(FILE_PREFIX) || !trimmed.ends_with(FILE_SUFFIX) {
            return Err(AiToolError::Execution(format!("file is invalid: {}", trimmed)));
        }
        let path = Self::log_dir()?.join(trimmed);
        if !path.is_file() {
            return Err(AiToolError::Execution(format!("log file not found: {}", trimmed)));
        }
        Ok(path)
    }

    fn page_rows(matched: Vec<String>, offset: usize, topn: usize) -> Result<QueryPage<String>, AiToolError> {
        let total = matched.len();
        if offset > 0 && offset >= total {
            return Err(AiToolError::Execution(format!("offset past end of log matches, total lines={}", total)));
        }
        let rows: Vec<String> = matched.into_iter().skip(offset).take(topn).collect();
        Ok(QueryPage { total, rows, page_size: topn, page_number: offset / topn })
    }

    async fn read_file(path: PathBuf, tokens: &[String]) -> Result<(Vec<String>, Vec<String>), AiToolError> {
        let content = tokio::fs::read_to_string(&path).await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::InvalidData {
                AiToolError::Execution(format!("file is not valid UTF-8: {}", path.display()))
            } else {
                AiToolError::Execution(format!("{}: {}", path.display(), error))
            }
        })?;
        let mut prefixes = BTreeSet::new();
        let mut matched = Vec::new();
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            if let Some(prefix) = Self::extract_prefix(line) {
                prefixes.insert(prefix);
            }
            if Self::line_matches(line, tokens) {
                matched.push(line.to_string());
            }
        }
        matched.reverse();
        Ok((prefixes.into_iter().collect(), matched))
    }
}

impl Tool for ToolLogList {
    const NAME: &'static str = AiToolId::ToolLogList.as_str();

    type Error = AiToolError;
    type Args = ToolLogListArgs;
    type Output = ToolLogListOut;

    fn description(&self) -> String {
        "查询本应用日志。先不传 file，rows 是日志文件名（新的在前），只列目录不读内容。再把选中的文件名传给 file，只读那一个文件。keyword 是任意子串，大小写不敏感；列文件时滤文件名，读文件时滤行，没有模块前缀的行也会命中。prefixes 只在读某个 file 时从该文件抽出，可选用不要每次都加前缀。total=0 时放宽：去掉括号、换短词、或不传 keyword。offset 跳过已看过的命中，topn 默认 10 上限 100。下一页 offset 用本次 offset + 本次 rows 条数。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "日志文件名，必须是本工具先列出来的那个名字。不传则只返回文件列表，不读任何日志内容。"
                },
                "keyword": {
                    "type": "string",
                    "description": "可选。任意子串，不必写成模块前缀。列文件时匹配文件名，读文件时匹配行。空格分隔的多个词需同时命中。搜不到就放宽或去掉本参数。"
                },
                "offset": {
                    "type": "integer",
                    "description": "跳过的命中条数，从 0 开始，默认 0。",
                    "default": 0
                },
                "topn": {
                    "type": "integer",
                    "description": "本次返回条数，默认 10，上限 100。",
                    "default": 10
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.topn == 0 {
            return Err(AiToolError::Execution("topn is 0".to_string()));
        }
        let topn = args.topn.min(TOPN_MAX);
        let tokens = Self::tokens(args.keyword.as_deref());
        let file = match args.file.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
            Some(name) => Some(name.to_string()),
            None => None,
        };
        let (prefixes, matched) = match file {
            None => {
                let names = Self::list_names()?;
                let filtered: Vec<String> = if tokens.is_empty() { names } else { names.into_iter().filter(|name| Self::line_matches(name, &tokens)).collect() };
                (Vec::new(), filtered)
            }
            Some(name) => {
                let path = Self::resolve_file(&name)?;
                Self::read_file(path, &tokens).await?
            }
        };
        let page = Self::page_rows(matched, args.offset, topn)?;
        let out = ToolLogListOut { prefixes, offset: args.offset, page };
        let body = serde_json::to_string(&out).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(out)
    }
}
