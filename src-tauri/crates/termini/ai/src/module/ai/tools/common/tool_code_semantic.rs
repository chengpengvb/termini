use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use rig::prelude::Tool;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use rig::tool::{DynamicTool, ToolContext};
use crate::module::ai::code_index::embed_client;
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::code_index::{resolve_search_root, semantic, IndexScope, ScopeHit};
use crate::module::ai::code_index::index_builder::{search_index_scoped, CodeIndexBuilder, IndexStage};

const MAX_TOP_K: usize = 50;
const DEFAULT_TOP_K: usize = 20;
const READ_MIN_LINES: usize = 60;
const READ_CONTEXT_LINES: usize = 20;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolCodeSemanticArgs {
    pub root: String,
    pub query: String,

    #[serde(default)]
    pub top_k: Option<usize>,

    #[serde(default)]
    pub rebuild: Option<bool>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[derive(Clone)]
struct Progress {
    call_id: Option<String>,
    chunk_push: ToolCallChunkPush,
    run_context: Option<Arc<AiRunContext>>,
}

impl Progress {
    fn emit(&self, stage: IndexStage) {
        let (Some(context), Some(call_id)) = (self.run_context.as_ref(), self.call_id.as_ref()) else {
            return;
        };
        let payload = match serde_json::to_string(&stage) {
            Ok(payload) => payload,
            Err(error) => {
                warn!("[ai] 编码进度失败 error={:#}", error);
                return;
            }
        };
        if let Err(error) = self.chunk_push.push(context, call_id, AiToolId::ToolCodeSemantic.as_str(), format!("{}\n", payload)) {
            warn!("[ai] 推送进度失败 call_id={} error={:#}", call_id, error);
        }
    }
}

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct ToolCodeSemantic {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    code_index_builder: CodeIndexBuilder,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,
}

#[async_trait]
impl BaseApp for ToolCodeSemantic {
    fn name(&self) -> &'static str {
        "ToolCodeSemantic"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolCodeSemantic {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolCodeSemantic
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::MAIN_READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolCodeSemantic {
    const NAME: &'static str = AiToolId::ToolCodeSemantic.as_str();

    type Args = ToolCodeSemanticArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "对本机代码或长文档做语义检索：用一句自然语言描述你要找的东西，直接返回最相关的位置（路径:起止行 + 符号名 + 片段 + 相似度）。**面对陌生项目、概念性问题（某机制在哪实现、某流程怎么走、按什么规律组织、文档里怎么说的）时这是首选工具**，一次调用就能拿到跨模块的候选位置，不必先建目录图再逐个翻。确切的符号名、报错原文、字面量才用 code_search。root 可以是仓库目录、其中任意子目录，也可以直接给单个大文件（一份长文档/长日志里定位内容就这么用）。索引以设置里配置的目录为单位，在后台建好并随文件改动自动更新；只要 root 落在这些目录之内或之上，都直接复用现成索引、一块都不重编，是秒级的，所以不必先找仓库根再查，想查哪层就传哪层。只有完全在配置范围之外的目录，首次调用才需要现建。多查几次几乎没有额外成本，不要因为怕慢而回避它。命中后用 file_read 读完整实现。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "绝对路径：仓库/项目根目录，或单个大文件（只索引这一个文件，秒级）"
                },
                "query": {
                    "type": "string",
                    "description": "自然语言描述要找什么，如「上下文超限时触发压缩的判定逻辑」"
                },
                "top_k": {
                    "type": "integer",
                    "description": "返回条数，默认 20，上限 50"
                },
                "rebuild": {
                    "type": "boolean",
                    "description": "强制重建索引，仅在用户明确说明代码已大改时传 true"
                }
            },
            "required": ["root", "query"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.get::<Arc<AiRunContext>>().cloned();
        let body = self.run(args, run_context).await?;
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolCodeSemantic {
    pub async fn search_existing(&self, root: &std::path::Path, query: &str, top_k: usize) -> Result<Option<String>> {
        let IndexScope::Covered(targets) = self.code_index_builder.resolve_scope(root).await else {
            return Ok(None);
        };
        let target = self.code_index_builder.resolve_target().await?;
        let silent = |_done: usize, _total: usize| {};
        let mut embedded_query = embed_client::embed_all(&target.provide, &[query.to_string()], 1, &silent).await?;
        if embedded_query.is_empty() {
            return Ok(None);
        }
        let dim = target.dim;
        let vector = embedded_query.remove(0);
        let mut merged: Vec<(f32, semantic::ChunkRow)> = Vec::new();
        for scope_hit in &targets {
            let dir = CodeIndexBuilder::index_dir(&scope_hit.index_root)?;
            let prefix = scope_hit.prefix.clone();
            let query_vector = vector.clone();
            let (rows, hits) = tokio::task::spawn_blocking(move || search_index_scoped(&dir, dim, &query_vector, top_k, prefix.as_ref())).await??;
            for (index, score) in hits {
                if let Some(row) = rows.get(index) {
                    merged.push((score, row.clone()));
                }
            }
        }
        merged.sort_by(|left, right| right.0.total_cmp(&left.0));
        merged.truncate(top_k);
        if merged.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.render(root, &targets, &merged, 0, true)))
    }

    async fn run(&self, args: ToolCodeSemanticArgs, run_context: Option<Arc<AiRunContext>>) -> Result<String, AiToolError> {
        let query = args.query.trim().to_string();
        if query.is_empty() {
            return Err(AiToolError::Execution("query is empty".to_string()));
        }
        let progress = Progress { call_id: args.internal_call_id.clone(), chunk_push: self.tool_call_chunk_push.clone(), run_context };
        let request = resolve_search_root(&args.root).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let force = args.rebuild.unwrap_or(false);

        let scope = self.code_index_builder.resolve_scope(&request).await;
        let (targets, reused) = match scope {
            IndexScope::Covered(hits) if !force => (hits, true),
            _ => (vec![ScopeHit { index_root: request.clone(), prefix: None }], false),
        };

        let stage_sink = |stage: IndexStage| progress.emit(stage);
        let mut embedded = 0usize;
        for target in &targets {
            embedded += self.code_index_builder.ensure_index(&target.index_root, force, &stage_sink).await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        }

        let target = self.code_index_builder.resolve_target().await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        let silent = |_done: usize, _total: usize| {};
        let mut embedded_query = embed_client::embed_all(&target.provide, &[query], 1, &silent).await.map_err(|error| AiToolError::Execution(format!("failed to embed query: {:#}", error)))?;
        if embedded_query.is_empty() {
            return Err(AiToolError::Execution("query embedding returned empty".to_string()));
        }
        let dim = target.dim;
        let vector = embedded_query.remove(0);
        let top_k = args.top_k.unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K);

        let mut merged: Vec<(f32, semantic::ChunkRow)> = Vec::new();
        for scope_hit in &targets {
            let dir = CodeIndexBuilder::index_dir(&scope_hit.index_root).map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
            let prefix = scope_hit.prefix.clone();
            let query_vector = vector.clone();
            let (rows, hits) = tokio::task::spawn_blocking(move || search_index_scoped(&dir, dim, &query_vector, top_k, prefix.as_ref()))
                .await
                .map_err(|error| AiToolError::Execution(format!("failed to schedule search: {}", error)))?
                .map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
            for (index, score) in hits {
                if let Some(row) = rows.get(index) {
                    merged.push((score, row.clone()));
                }
            }
        }
        merged.sort_by(|left, right| right.0.total_cmp(&left.0));
        merged.truncate(top_k);
        if merged.is_empty() {
            return Ok(format!("[code_semantic] {}\n(no matches)", request.display()));
        }
        Ok(self.render(&request, &targets, &merged, embedded, reused))
    }

    fn render(&self, request: &std::path::Path, targets: &[ScopeHit], merged: &[(f32, semantic::ChunkRow)], embedded: usize, reused: bool) -> String {
        let mut out = format!("[code_semantic] {} (命中 {} 条, 本次新编码 {} 块)\n", request.display(), merged.len(), embedded);
        if reused {
            for scope_hit in targets {
                match &scope_hit.prefix {
                    Some(prefix) => out.push_str(&format!("(复用已建索引 {} 并收窄到 {})\n", scope_hit.index_root.display(), prefix.trim_end_matches('/'))),
                    None if scope_hit.index_root != request => out.push_str(&format!("(复用已建索引 {}; 该目录以外未纳入索引)\n", scope_hit.index_root.display())),
                    None => {}
                }
            }
        }
        let mut grouped: Vec<(&str, Vec<&(f32, semantic::ChunkRow)>)> = Vec::new();
        for item in merged {
            match grouped.iter_mut().find(|(path, _)| *path == item.1.path.as_str()) {
                Some((_, bucket)) => bucket.push(item),
                None => grouped.push((item.1.path.as_str(), vec![item])),
            }
        }
        for (path, bucket) in &grouped {
            out.push_str(&format!("{}\n", path));
            for (score, row) in bucket {
                out.push_str(&format!("  {}-{} {} score={:.3}\n    {}\n", row.start_line, row.end_line, row.symbol, score, row.preview));
            }
        }
        if grouped.len() > 1 {
            let index_root = targets.first().map(|hit| hit.index_root.display().to_string()).unwrap_or_else(|| request.display().to_string());
            let entries: Vec<String> = grouped
                .iter()
                .map(|(path, bucket)| {
                    let start = bucket.iter().map(|(_, row)| row.start_line).min().unwrap_or(1).saturating_sub(READ_CONTEXT_LINES).max(1);
                    let end = bucket.iter().map(|(_, row)| row.end_line).max().unwrap_or(start) + READ_CONTEXT_LINES;
                    format!("{{\"path\":\"{}\",\"offset\":{},\"limit\":{}}}", path, start, (end - start + 1).max(READ_MIN_LINES))
                })
                .collect();
            out.push_str(&format!("\n要看实现, 直接一次读完全部 {} 个文件, 不要两个两个地读:\nfile_read root=\"{}\" files=[{}]\n", grouped.len(), index_root, entries.join(",")));
        }
        out
    }
}
