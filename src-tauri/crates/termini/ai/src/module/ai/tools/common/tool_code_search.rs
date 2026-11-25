use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::code_index::walker::SearchHit;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::tool_code_semantic::ToolCodeSemantic;
use crate::module::ai::code_index::{render_group, resolve_search_root, search_files_multi};

const MAX_QUERIES: usize = 10;
const SEMANTIC_FALLBACK_TOP_K: usize = 8;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolCodeSearchArgs {
    pub root: String,

    #[serde(default)]
    pub glob: Option<String>,

    #[serde(default)]
    pub query: Option<String>,

    #[serde(default)]
    pub max_matches: Option<usize>,

    #[serde(default)]
    pub queries: Option<Vec<String>>,
}

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct ToolCodeSearch {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_code_semantic: ToolCodeSemantic,
}

#[async_trait]
impl BaseApp for ToolCodeSearch {
    fn name(&self) -> &'static str {
        "ToolCodeSearch"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolCodeSearch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolCodeSearch
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolCodeSearch {
    const NAME: &'static str = AiToolId::ToolCodeSearch.as_str();

    type Error = AiToolError;
    type Args = ToolCodeSearchArgs;
    type Output = String;

    fn description(&self) -> String {
        "在本机代码仓库里按正则/文本检索，返回 path:line:text，命中行上下各带 3 行上下文（缩进带行号），看清签名、调用方式、注解通常到此为止，不需要再 file_read 一趟去看同一处。尊重 .gitignore。每个文件最多返回 3 条，让结果铺开到更多文件而不是堆在一个文件里。**要查多个关键字时一次传 queries 数组，不要一个个来回调用**：只遍历目录树一遍、每个文件只读一次，结果按关键字分组（## 关键字 段落）返回，比逐个搜快且省往返。每个关键字有各自的命中额度。用于按关键词、报错信息定位，再决定读哪些文件；接着可以把结果里的 path 和行号填进 file_read 的 files 批量取实现。"
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "绝对路径。可以是仓库/项目/子目录，也可以直接是单个文件（只在该文件里搜）"
                },
                "queries": {
                    "type": "array",
                    "description": "多关键字批量检索，结果按关键字分组。查多个词优先用它，只走一遍目录树",
                    "items": {"type": "string"}
                },
                "query": {
                    "type": "string",
                    "description": "单个正则表达式或关键字。与 queries 二选一"
                },
                "glob": {
                    "type": "string",
                    "description": "可选文件过滤，如 **/*.rs 或 src/**/*.ts"
                },
                "max_matches": {
                    "type": "integer",
                    "description": "每个关键字最多返回条数，默认 50，上限 200"
                }
            },
            "required": ["root"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let queries = Self::resolve_queries(&args)?;
        let root = resolve_search_root(&args.root).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let glob = args.glob;
        let max_matches = args.max_matches;
        let root_search = root.clone();
        let search_queries = queries.clone();
        let grouped = tokio::task::spawn_blocking(move || search_files_multi(&root_search, &search_queries, glob.as_deref(), max_matches))
            .await
            .map_err(|error| AiToolError::Execution(error.to_string()))?
            .map_err(|error| AiToolError::Execution(error.to_string()))?;
        if grouped.len() != queries.len() {
            return Err(AiToolError::Execution(format!("search groups {} != queries {}", grouped.len(), queries.len())));
        }
        let all_empty = grouped.iter().all(Vec::is_empty);
        let fallback = if all_empty { self.semantic_fallback(&root, &queries).await } else { String::new() };
        let last = queries.len() - 1;
        let mut items: Vec<WrapItem> = Vec::with_capacity(queries.len());
        for (index, query) in queries.iter().enumerate() {
            let mut item = Self::wrap_query(&root, query, &grouped[index]);
            if all_empty && index == last {
                item.body.push_str(&fallback);
            }
            items.push(item);
        }
        context.insert_result(items);
        Ok(String::new())
    }
}

impl ToolCodeSearch {
    fn wrap_query(root: &std::path::Path, query: &str, hits: &[SearchHit]) -> WrapItem {
        let title = if hits.is_empty() {
            format!("## {}", query)
        } else {
            let files = hits.iter().map(|hit| hit.path.as_path()).collect::<std::collections::HashSet<_>>().len();
            format!("## {} ({} matches in {} files)", query, hits.len(), files)
        };
        WrapItem::wrap_item(Some(title), render_group(root, hits))
    }

    async fn semantic_fallback(&self, root: &std::path::Path, queries: &[String]) -> String {
        let text = queries.join(" ");
        match self.tool_code_semantic.search_existing(root, &text, SEMANTIC_FALLBACK_TOP_K).await {
            Ok(Some(rendered)) => format!("\nno exact matches. Below are the semantically closest locations in the same scope (the index was already built, so this was a free lookup); if this is what you were looking for, read them directly instead of re-searching the same keywords in another directory:\n{}", rendered),
            Ok(None) => "\nno exact matches, and no semantic index is available for this scope. These names are probably not in this scope: if they are symbols mentioned in the prompt, just state in the report that they were not found; if you are looking for a concept, ask code_semantic a natural-language question about the directory.\n"
                .to_string(),
            Err(error) => {
                warn!("[ai] 语义兜底失败 root={} error={:#}", root.display(), error);
                String::new()
            }
        }
    }

    fn resolve_queries(args: &ToolCodeSearchArgs) -> Result<Vec<String>, AiToolError> {
        let mut queries: Vec<String> = Vec::new();
        if let Some(list) = args.queries.as_ref() {
            queries.extend(list.iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()));
        }
        if let Some(single) = args.query.as_ref().map(|item| item.trim()).filter(|item| !item.is_empty()) {
            if !queries.iter().any(|item| item == single) {
                queries.push(single.to_string());
            }
        }
        if queries.is_empty() {
            return Err(AiToolError::Execution("query is empty: pass query=\"...\" or queries=[\"...\"]".to_string()));
        }
        if queries.len() > MAX_QUERIES {
            return Err(AiToolError::Execution(format!("too many queries: {} exceeds {}", queries.len(), MAX_QUERIES)));
        }
        Ok(queries)
    }
}
