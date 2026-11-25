use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ChatFileStore;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::shared::chat_file_store::CHAT_OUTPUTS_DIR;

const MAX_LINES: usize = 1000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolOutputReadArgs {
    pub id: String,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,
}

#[base_app(sort = 777)]
#[derive(Clone, Service)]
pub struct ToolOutputRead {
    #[inject(component)]
    tool_manager: ToolManager,
}

#[async_trait]
impl BaseApp for ToolOutputRead {
    fn name(&self) -> &'static str {
        "ToolOutputRead"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolOutputRead {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolOutputRead
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

impl ToolOutputRead {
    fn resolve_id(id: &str) -> Result<String, AiToolError> {
        let id = id.trim();
        if id.is_empty() {
            return Err(AiToolError::Execution("id is empty".to_string()));
        }
        if !id.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            return Err(AiToolError::Execution(format!("id is invalid: {}", id)));
        }
        Ok(id.to_string())
    }

    fn window(content: &str, offset: Option<usize>, limit: Option<usize>) -> Result<String, AiToolError> {
        if offset.is_none() && limit.is_none() {
            return Ok(content.to_string());
        }
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        if total == 0 {
            return Ok(String::new());
        }
        let start = match offset {
            None => 0,
            Some(0) => return Err(AiToolError::Execution("offset starts at 1".to_string())),
            Some(value) => value - 1,
        };
        if start >= total {
            return Err(AiToolError::Execution(format!("offset past end of truncated output, total lines={}", total)));
        }
        let take = match limit {
            None => MAX_LINES,
            Some(0) => return Err(AiToolError::Execution("limit is 0".to_string())),
            Some(value) => value.min(MAX_LINES),
        };
        let end = (start + take).min(total);
        let mut body = lines[start..end].join("\n");
        if end < total {
            body.push_str(&format!("\n… truncated, showing lines {}-{} of {}", start + 1, end, total));
        }
        Ok(body)
    }
}

impl Tool for ToolOutputRead {
    const NAME: &'static str = AiToolId::ToolOutputRead.as_str();

    type Error = AiToolError;
    type Args = ToolOutputReadArgs;
    type Output = String;

    fn description(&self) -> String {
        "查阅工具输出被掐头去尾后落下的全文快照。只接受截断回执里的 [truncated id=…] 那个 id，读的是当时那一条输出，不会重跑原工具。要看中间或后段用 offset（从 1 起的行号）和 limit（最多行数，默认最多 1000 行），看 showing lines X-Y of Z 继续读。没有 truncated id 的不要用本工具，缩小原工具参数再查。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "[truncated id=…] 里的 id"
                },
                "offset": {
                    "type": "integer",
                    "description": "起始行号，从 1 开始；不传则从第 1 行"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多返回行数，默认最多 1000 行"
                }
            },
            "required": ["id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(chat_id) = run_context.chat_id.as_deref() else {
            return Err(AiToolError::Execution("chat_id is missing".to_string()));
        };
        let id = Self::resolve_id(&args.id)?;
        let path = ChatFileStore::chat_dir(chat_id).map_err(|error| AiToolError::Execution(error.to_string()))?.join(CHAT_OUTPUTS_DIR).join(format!("{}.txt", id));
        if !path.exists() {
            return Err(AiToolError::Execution(format!("truncated output not found: {}", id)));
        }
        if !path.is_file() {
            return Err(AiToolError::Execution(format!("not a file: {}", path.display())));
        }
        let content = tokio::fs::read_to_string(&path).await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::InvalidData {
                AiToolError::Execution(format!("file is not valid UTF-8: {}", path.display()))
            } else {
                AiToolError::Execution(format!("{}: {}", path.display(), error))
            }
        })?;
        let body = Self::window(&content, args.offset, args.limit)?;
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
