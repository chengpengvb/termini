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
use crate::module::ai::tools::common::document::external_tool::ExternalTool;
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;
use crate::module::ai::tools::common::document::markdown_source::MarkdownInput;
use crate::module::ai::tools::common::document::markdown_normalize::{self, DocTarget};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolPdfWriteArgs {
    pub path: String,

    #[serde(flatten)]
    pub input: MarkdownInput,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolPdfWrite {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_installer: ToolInstaller,

    #[inject(component)]
    pandoc_converter: PandocConverter,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolPdfWrite {
    fn name(&self) -> &'static str {
        "ToolPdfWrite"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolPdfWrite {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolPdfWrite
    }

    fn deferred(&self) -> bool {
        true
    }

    fn platform_supported(&self) -> bool {
        !cfg!(any(target_os = "ios", target_os = "android"))
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolPdfWrite {
    const NAME: &'static str = AiToolId::ToolPdfWrite.as_str();

    type Args = ToolPdfWriteArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "把 Markdown 用 pandoc + typst 排版成 .pdf 写到本应用所在宿主机（A4，自动分页，中日韩用本机字体），适合文字型文档的快捷导出。Markdown 必须是可交付成品：标题层级、摘要或结论、分段、表格，不要无结构粘贴。内容来源二选一：source 传一个已有 Markdown 文件的绝对路径（推荐，例如 image_read/pdf_read 等保存的派生 md 或你用 file_write 写好的 md，不要把文件内容抄进参数），或 markdown 直接传文本。标题、段落、换行、列表、引用、表格、代码块都按 Markdown 结构排版。需要自定义版式、合并/拆分/加水印或修改已有 PDF 时，改用 script_run 写脚本（pdf-lib 或 reportlab/pypdf）。首次使用会自动下载 pandoc 和 typst（合计几十 MB，可能要等一会）。执行前会请求用户确认，成功后覆盖目标文件。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "输出 .pdf 文件的绝对路径" },
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
        let path = markdown_source::resolve_output(&args.path, "pdf")?;
        let normalized = markdown_normalize::normalize(&markdown, args.input.base_dir().as_deref(), DocTarget::Pdf);
        let (report, check) = {
            let _guard = self.tool_manager.lock_path(&ToolManager::local_path_key(&path)).await;
            let report = self.pandoc_converter.markdown_to_pdf(&normalized.markdown, &path).await?;
            (report, self.read_back(&path).await)
        };
        let body = markdown_source::describe_result("pdf_write", &report, &normalized.issues, check.as_ref());
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolPdfWrite {
    async fn read_back(&self, path: &Path) -> Option<output_check::DocCheck> {
        let dll = match self.tool_installer.ensure(ExternalTool::Pdfium).await {
            Ok(dll) => dll,
            Err(error) => {
                warn!("[ai] 回读缺少 pdfium path={} error={}", path.display(), error);
                return None;
            }
        };
        let target = path.to_path_buf();
        match tokio::task::spawn_blocking(move || output_check::check_pdf(&dll, &target)).await {
            Ok(Ok(check)) => Some(check),
            Ok(Err(error)) => {
                warn!("[ai] 回读失败 path={} error={}", path.display(), error);
                None
            }
            Err(error) => {
                warn!("[ai] 回读任务异常 path={} error={}", path.display(), error);
                None
            }
        }
    }
}
