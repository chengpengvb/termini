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
use crate::module::ai::code_index::{render_symbol, resolve_root, scan_bases, scope_from_dirs, CodeIndex};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolCodeSymbolArgs {
    pub root: String,
    pub name: String,

    #[serde(default)]
    pub kind: Option<String>,

    #[serde(default)]
    pub offset: Option<usize>,
}

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct ToolCodeSymbol {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    code_index: CodeIndex,

    #[inject(component)]
    config_service: ConfigService,
}

#[async_trait]
impl BaseApp for ToolCodeSymbol {
    fn name(&self) -> &'static str {
        "ToolCodeSymbol"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolCodeSymbol {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolCodeSymbol
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

impl Tool for ToolCodeSymbol {
    const NAME: &'static str = AiToolId::ToolCodeSymbol.as_str();

    type Error = AiToolError;
    type Args = ToolCodeSymbolArgs;
    type Output = String;

    fn description(&self) -> String {
        "按符号名查找本机仓库中的定义位置，并附带同文件内引用。名字支持子串匹配，精确命中排在前面，记不全名字也能查。每次最多 8 条定义，返回 total；total 大于本页时用 offset 继续翻，offset 默认 0。用于替代盲目翻文件猜函数在哪。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "仓库/项目根目录绝对路径"
                },
                "name": {
                    "type": "string",
                    "description": "符号名，如函数/结构体/类名"
                },
                "kind": {
                    "type": "string",
                    "description": "可选过滤：function/struct/class/trait/method/type 等"
                },
                "offset": {
                    "type": "integer",
                    "description": "从第几条定义开始，默认 0；每次 8 条，看 total 继续翻"
                }
            },
            "required": ["root", "name"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let name = args.name;
        let kind = args.kind;
        let index = self.code_index.clone();
        let offset = args.offset.unwrap_or(0);
        let root = resolve_root(&args.root).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let scope = scope_from_dirs(&root, &self.config_service.select_index_dirs().await);
        let bases = scan_bases(&root, &scope);
        let body = tokio::task::spawn_blocking(move || {
            let symbols = index.symbols_for_bases(&root, &bases).map_err(|error| error.to_string())?;
            Ok::<String, String>(render_symbol(&root, &symbols, &name, kind.as_deref(), offset))
        })
        .await
        .map_err(|error| AiToolError::Execution(error.to_string()))?
        .map_err(AiToolError::Execution)?;
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
