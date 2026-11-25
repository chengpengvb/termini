use anyhow::Result;
use std::sync::Arc;
use std::path::Path;
use serde_json::json;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::document::output_check;
use crate::module::ai::tools::common::document::markdown_source;
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::document::pandoc::PandocConverter;
use crate::module::ai::tools::base::{PathGuard, ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::document::markdown_source::MarkdownInput;
use crate::module::ai::tools::common::document::markdown_normalize::{self, DocTarget};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolDocxWriteArgs {
    pub path: String,

    #[serde(flatten)]
    pub input: MarkdownInput,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolDocxWrite {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    pandoc_converter: PandocConverter,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolDocxWrite {
    fn name(&self) -> &'static str {
        "ToolDocxWrite"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDocxWrite {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDocxWrite
    }

    fn platform_supported(&self) -> bool {
        !cfg!(any(target_os = "ios", target_os = "android"))
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolDocxWrite {
    const NAME: &'static str = AiToolId::ToolDocxWrite.as_str();

    type Args = ToolDocxWriteArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "把 Markdown 用 pandoc 转成 .docx 写到本应用所在宿主机，适合文字型文档（方案、报告、试卷、转写稿）的快捷导出。Markdown 必须是可交付成品：标题层级、摘要或结论、分段、表格，不要无结构粘贴。内容来源二选一：source 传一个已有 Markdown 文件的绝对路径（推荐，例如 image_read/pdf_read 等保存的派生 md 或你用 file_write 写好的 md，不要把文件内容抄进参数），或 markdown 直接传文本。标题、段落、换行、列表、引用、表格、代码块、图片链接都按 Markdown 结构转换成真正的 Word 样式。需要封面、页眉页脚、分栏、指定字体字号等版式控制，或要修改已有 docx 时，改用 script_run 写脚本（docx 或 python-docx）。首次使用会自动下载 pandoc（几十 MB，可能要等一会）。执行前会请求用户确认，成功后覆盖目标文件。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "输出 .docx 文件的绝对路径" },
                "source": { "type": "string", "description": "Markdown 源文件的绝对路径，与 markdown 二选一，优先用这个" },
                "markdown": { "type": "string", "description": "直接给出的 Markdown 文本，没有现成文件时才用" }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        PathGuard::ensure_writable(Path::new(args.path.trim()))?;
        let markdown = args.input.load()?;
        let confirm = args.input.confirm(&args.path);
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm).await?;
        let path = markdown_source::resolve_output(&args.path, "docx")?;
        let normalized = markdown_normalize::normalize(&markdown, args.input.base_dir().as_deref(), DocTarget::Docx);
        let (report, check) = {
            let _guard = self.tool_manager.lock_path(&ToolManager::local_path_key(&path)).await;
            let report = self.pandoc_converter.markdown_to_docx(&normalized.markdown, &path).await?;
            let check = output_check::check_docx(&self.pandoc_converter, &path).await.map_err(|error| warn!("[ai] 回读失败 path={} error={}", path.display(), error)).ok();
            (report, check)
        };
        let body = markdown_source::describe_result("docx_write", &report, &normalized.issues, check.as_ref());
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
