use anyhow::Result;
use std::sync::Arc;
use std::path::PathBuf;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use serde_json::{json, Value as JsonValue};
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use calamine::{open_workbook_auto, Data, Reader};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::derived::{DerivedKind, DerivedStore};
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 2000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolXlsxReadArgs {
    pub path: String,

    #[serde(default)]
    pub sheet: Option<String>,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub refresh: Option<bool>,
}

const FULL_SHEET_MAX_ROWS: usize = 20000;

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolXlsxRead {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    derived_store: DerivedStore,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolXlsxRead {
    fn name(&self) -> &'static str {
        "ToolXlsxRead"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolXlsxRead {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolXlsxRead
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

impl ToolXlsxRead {
    fn resolve_path(path: &str) -> Result<PathBuf, AiToolError> {
        if path.trim().is_empty() {
            return Err(AiToolError::Execution("path is empty".to_string()));
        }
        let raw = PathBuf::from(path);
        if !raw.is_absolute() {
            return Err(AiToolError::Execution(format!("path must be absolute: {}", path)));
        }
        Ok(raw)
    }

    fn cell_to_json(data: &Data) -> JsonValue {
        match data {
            Data::Empty => JsonValue::Null,
            Data::String(text) => JsonValue::String(text.clone()),
            Data::Float(value) => serde_json::Number::from_f64(*value).map(JsonValue::Number).unwrap_or(JsonValue::Null),
            Data::Int(value) => JsonValue::Number((*value).into()),
            Data::Bool(value) => JsonValue::Bool(*value),
            Data::DateTime(value) => JsonValue::String(value.to_string()),
            Data::DateTimeIso(value) => JsonValue::String(value.clone()),
            Data::DurationIso(value) => JsonValue::String(value.clone()),
            Data::Error(error) => JsonValue::String(format!("#ERROR:{:?}", error)),
        }
    }

    fn cell_to_text(data: &Data) -> String {
        match data {
            Data::Empty => String::new(),
            Data::String(text) => text.replace('\n', " ").replace('|', "\\|"),
            Data::Float(value) => value.to_string(),
            Data::Int(value) => value.to_string(),
            Data::Bool(value) => value.to_string(),
            Data::DateTime(value) => value.to_string(),
            Data::DateTimeIso(value) => value.clone(),
            Data::DurationIso(value) => value.clone(),
            Data::Error(error) => format!("#ERROR:{:?}", error),
        }
    }

    fn render_markdown(path: &PathBuf) -> Result<String, String> {
        let mut wb = open_workbook_auto(path).map_err(|error| error.to_string())?;
        let sheet_names: Vec<String> = wb.sheet_names().to_owned();
        let mut out = String::new();
        for name in sheet_names {
            let Ok(range) = wb.worksheet_range(&name) else {
                continue;
            };
            let (rows_total, _) = range.get_size();
            out.push_str(&format!("## Sheet: {} ({} 行)\n\n", name, rows_total));
            for (index, row) in range.rows().enumerate() {
                if index >= FULL_SHEET_MAX_ROWS {
                    out.push_str(&format!("... 其余 {} 行未落盘, 用 xlsx_read sheet={} offset={} 读取\n", rows_total - index, name, index));
                    break;
                }
                let cells: Vec<String> = row.iter().map(ToolXlsxRead::cell_to_text).collect();
                out.push_str(&format!("| {} |\n", cells.join(" | ")));
                if index == 0 {
                    out.push_str(&format!("|{}\n", " --- |".repeat(cells.len().max(1))));
                }
            }
            out.push('\n');
        }
        Ok(out)
    }
}

impl Tool for ToolXlsxRead {
    const NAME: &'static str = AiToolId::ToolXlsxRead.as_str();

    type Error = AiToolError;
    type Args = ToolXlsxReadArgs;
    type Output = String;

    fn description(&self) -> String {
        "读取本应用所在宿主机上的 Excel 文件（.xlsx/.xls/.xlsm/.xlsb/.ods）。可指定 sheet（工作表名，不填时读第一张）、offset（起始行 0-based）、limit（最多返回行数，默认 200，最大 2000）。返回 JSON：{sheet_names, sheet, total_rows, rows}，rows 是二维数组，单元格类型按原始 JSON 类型返回（字符串/数字/布尔/null）。首次读取会把整个工作簿保存为一份 Markdown 全文（返回开头给出路径，每张表一节 `## Sheet: 名称`），之后翻页、用 code_search/code_semantic 找内容可直接对那个路径操作；文件内容变了自动重新解析，也可传 refresh=true 强制。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Excel 文件绝对路径" },
                "sheet": { "type": "string", "description": "工作表名；不传时使用第一张" },
                "offset": { "type": "integer", "description": "起始行 0-based，默认 0" },
                "limit": { "type": "integer", "description": "最多返回行数，默认 200，最大 2000" },
                "refresh": { "type": "boolean", "description": "true 忽略已保存的全文重新解析；默认 false" }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let path = Self::resolve_path(&args.path)?;
        if !path.exists() {
            return Err(AiToolError::Execution(format!("file not found: {}", path.display())));
        }
        if !path.is_file() {
            return Err(AiToolError::Execution(format!("not a file: {}", path.display())));
        }
        let meta = std::fs::metadata(&path).map_err(|error| AiToolError::Execution(error.to_string()))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path.display())));
        }
        let sheet = args.sheet.clone();
        let offset = args.offset.unwrap_or(0);
        let limit = args.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
        let full_path = path.clone();
        let (derived, _) = self
            .derived_store
            .ensure_file_document(DerivedKind::Xlsx, &path, args.refresh.unwrap_or(false), move || Self::render_markdown(&full_path))
            .await
            .map_err(|error| AiToolError::Execution(error.to_string()))?;
        if let Some(chat_id) = self.chat_session_cache_manager.chat_id(&run_context.req_id) {
            self.derived_store.record_material(&chat_id, &derived);
        }
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;

        let output = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let mut wb = open_workbook_auto(&path).map_err(|error| error.to_string())?;
            let sheet_names: Vec<String> = wb.sheet_names().to_owned();
            if sheet_names.is_empty() {
                return Err("workbook has no sheet".to_string());
            }
            let target = sheet.unwrap_or_else(|| sheet_names[0].clone());
            let range = wb.worksheet_range(&target).map_err(|error| format!("sheet `{}`: {}", target, error))?;
            let total_rows = range.get_size().0;
            let end = (offset + limit).min(total_rows);
            let mut rows: Vec<Vec<JsonValue>> = Vec::new();
            for (index, row) in range.rows().enumerate() {
                if index < offset {
                    continue;
                }
                if index >= end {
                    break;
                }
                rows.push(row.iter().map(ToolXlsxRead::cell_to_json).collect());
            }
            let payload = json!({
                "sheet_names": sheet_names,
                "sheet": target,
                "total_rows": total_rows,
                "offset": offset,
                "rows": rows,
            });
            serde_json::to_string(&payload).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| AiToolError::Execution(error.to_string()))?
        .map_err(AiToolError::Execution)?;
        let body = format!("{}\n\n{}", derived.header(&derived_root), output);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
