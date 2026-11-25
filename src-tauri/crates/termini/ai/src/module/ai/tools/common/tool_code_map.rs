use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::service::service_config::ConfigService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::code_index::{render_map, resolve_search_root, scan_bases, scope_from_dirs, CodeIndex};

const CODE_MAP_MAX_TOKENS: usize = 4096;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolCodeMapArgs {
    pub root: String,

    #[serde(default)]
    pub focus: Option<String>,

    #[serde(default)]
    pub max_tokens: Option<usize>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolCodeMap {
    #[inject(component)]
    code_index: CodeIndex,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    config_service: ConfigService,
}

#[async_trait]
impl BaseApp for ToolCodeMap {
    fn name(&self) -> &'static str {
        "ToolCodeMap"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolCodeMap {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolCodeMap
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

impl Tool for ToolCodeMap {
    const NAME: &'static str = AiToolId::ToolCodeMap.as_str();

    type Args = ToolCodeMapArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "生成本机代码仓库的压缩结构地图。小仓库直接列每个文件的定义；大仓库自动降到目录视图，按目录给出文件数、定义数和代表性符号，用来先看清模块划分再决定往哪深入。改代码或排障前先用它建立全局视图。root 必须是本机绝对目录；远程代码先 sftp_transfer_batch 拉到本机再调用。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "绝对路径。可以是仓库/项目/子目录，也可以直接是单个文件（只列该文件的定义）"
                },
                "focus": {
                    "type": "string",
                    "description": "可选，路径片段或符号名，命中的定义会优先排到前面"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "地图体积预算，默认 2048，上限 4096"
                }
            },
            "required": ["root"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let root = resolve_search_root(&args.root).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let scope = scope_from_dirs(&root, &self.config_service.select_index_dirs().await);
        let bases = scan_bases(&root, &scope);
        let index = self.code_index.clone();
        let focus = args.focus;
        let budget = args.max_tokens.map(|value| value.min(CODE_MAP_MAX_TOKENS));
        let body = tokio::task::spawn_blocking(move || {
            let symbols = index.symbols_for_bases(&root, &bases).map_err(|error| error.to_string())?;
            Ok::<String, String>(render_map(&root, &symbols, focus.as_deref(), budget))
        })
        .await
        .map_err(|error| AiToolError::Execution(error.to_string()))?
        .map_err(AiToolError::Execution)?;
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
