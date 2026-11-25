use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use std::path::PathBuf;
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
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::base::{PathGuard, ToolBaseOutWrap, WrapItem};
use rust_xlsxwriter::{Color, ConditionalFormatCell, ConditionalFormatCellRule, Format, Workbook, Worksheet};

const MAX_SHEETS: usize = 32;
const MAX_COLS_PER_ROW: usize = 512;
const NUMBER_FORMAT: &str = "#,##0.##";
const MAX_ROWS_PER_SHEET: usize = 100_000;
const HEADER_FILL: Color = Color::RGB(0xD9D9D9);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolXlsxWriteHighlight {
    pub op: String,
    pub value: f64,
    pub column: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolXlsxWriteSheet {
    pub name: String,
    pub rows: Vec<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<ToolXlsxWriteHighlight>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolXlsxWriteArgs {
    pub path: String,
    pub sheets: Vec<ToolXlsxWriteSheet>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

struct CellStyles {
    header: Format,
    number: Format,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolXlsxWrite {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolXlsxWrite {
    fn name(&self) -> &'static str {
        "ToolXlsxWrite"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolXlsxWrite {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolXlsxWrite
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

impl ToolXlsxWrite {
    fn resolve_path(path: &str) -> Result<PathBuf, AiToolError> {
        if path.trim().is_empty() {
            return Err(AiToolError::Execution("path is empty".to_string()));
        }
        let raw = PathBuf::from(path);
        if !raw.is_absolute() {
            return Err(AiToolError::Execution(format!("path must be absolute: {}", path)));
        }
        if !raw.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("xlsx")) {
            return Err(AiToolError::Execution(format!("path must end with .xlsx: {}", path)));
        }
        Ok(raw)
    }

    fn write_cell(worksheet: &mut Worksheet, row: u32, col: u16, value: &serde_json::Value, styles: &CellStyles) -> Result<(), String> {
        let header = row == 0;
        let result = match value {
            serde_json::Value::Null => {
                if header {
                    worksheet.write_blank(row, col, &styles.header)
                } else {
                    return Ok(());
                }
            }
            serde_json::Value::Bool(inner) => {
                if header {
                    worksheet.write_boolean_with_format(row, col, *inner, &styles.header)
                } else {
                    worksheet.write_boolean(row, col, *inner)
                }
            }
            serde_json::Value::Number(number) => match number.as_f64() {
                Some(number) if header => worksheet.write_number_with_format(row, col, number, &styles.header),
                Some(number) => worksheet.write_number_with_format(row, col, number, &styles.number),
                None if header => worksheet.write_string_with_format(row, col, number.to_string().as_str(), &styles.header),
                None => worksheet.write_string(row, col, number.to_string().as_str()),
            },
            serde_json::Value::String(text) => {
                if header {
                    worksheet.write_string_with_format(row, col, text.as_str(), &styles.header)
                } else {
                    worksheet.write_string(row, col, text.as_str())
                }
            }
            other => {
                let text = other.to_string();
                if header {
                    worksheet.write_string_with_format(row, col, text.as_str(), &styles.header)
                } else {
                    worksheet.write_string(row, col, text.as_str())
                }
            }
        };
        result.map(|_| ()).map_err(|error| error.to_string())
    }

    fn highlight_rule(highlight: &ToolXlsxWriteHighlight) -> Result<ConditionalFormatCellRule<f64>, String> {
        match highlight.op.trim() {
            ">" => Ok(ConditionalFormatCellRule::GreaterThan(highlight.value)),
            "<" => Ok(ConditionalFormatCellRule::LessThan(highlight.value)),
            "==" | "=" => Ok(ConditionalFormatCellRule::EqualTo(highlight.value)),
            other => Err(format!("unsupported highlight op `{}`, expected one of >, <, ==", other)),
        }
    }

    fn apply_highlight(worksheet: &mut Worksheet, highlight: &ToolXlsxWriteHighlight, data_rows: usize) -> Result<(), String> {
        if data_rows == 0 {
            return Ok(());
        }
        if highlight.column >= MAX_COLS_PER_ROW {
            return Err(format!("highlight column {} exceeds limit {}", highlight.column, MAX_COLS_PER_ROW));
        }
        let rule = Self::highlight_rule(highlight)?;
        let format = ConditionalFormatCell::new().set_rule(rule).set_format(Format::new().set_font_color(Color::Red));
        let col = highlight.column as u16;
        worksheet.add_conditional_format(1, col, data_rows as u32, col, &format).map(|_| ()).map_err(|error| error.to_string())
    }

    fn build_workbook(sheets: &[ToolXlsxWriteSheet]) -> Result<Workbook, String> {
        let styles = CellStyles { header: Format::new().set_bold().set_background_color(HEADER_FILL), number: Format::new().set_num_format(NUMBER_FORMAT) };
        let mut wb = Workbook::new();
        for sheet in sheets {
            if sheet.name.trim().is_empty() {
                return Err("sheet name is empty".to_string());
            }
            if sheet.rows.len() > MAX_ROWS_PER_SHEET {
                return Err(format!("sheet `{}` rows {} exceed limit {}", sheet.name, sheet.rows.len(), MAX_ROWS_PER_SHEET));
            }
            let worksheet = wb.add_worksheet();
            worksheet.set_name(sheet.name.as_str()).map_err(|error| format!("sheet `{}`: {}", sheet.name, error))?;
            for (row_index, row) in sheet.rows.iter().enumerate() {
                if row.len() > MAX_COLS_PER_ROW {
                    return Err(format!("sheet `{}` row {} cols {} exceed limit {}", sheet.name, row_index, row.len(), MAX_COLS_PER_ROW));
                }
                for (col_index, cell) in row.iter().enumerate() {
                    Self::write_cell(worksheet, row_index as u32, col_index as u16, cell, &styles).map_err(|error| format!("sheet `{}` row {} col {}: {}", sheet.name, row_index, col_index, error))?;
                }
            }
            if sheet.rows.is_empty() {
                continue;
            }
            worksheet.set_freeze_panes(1, 0).map_err(|error| format!("sheet `{}`: {}", sheet.name, error))?;
            if let Some(highlight) = sheet.highlight.as_ref() {
                Self::apply_highlight(worksheet, highlight, sheet.rows.len() - 1).map_err(|error| format!("sheet `{}` highlight: {}", sheet.name, error))?;
            }
            worksheet.autofit();
        }
        Ok(wb)
    }
}

impl Tool for ToolXlsxWrite {
    const NAME: &'static str = AiToolId::ToolXlsxWrite.as_str();

    type Error = AiToolError;
    type Args = ToolXlsxWriteArgs;
    type Output = String;

    fn description(&self) -> String {
        "在本应用所在宿主机上写入或覆盖 .xlsx 文件，适合纯数据表的快捷导出。表头用真实列名和单位，多类数据分表，不要无表头裸数字。sheets 为多张工作表，每张工作表的 rows 是二维数组；单元格类型按 JSON 类型自动写入：string→文本、number→数字、boolean→布尔、null→空单元格。第一行按表头处理（加粗、浅灰底、冻结），列宽自动适配，数字列保留数值类型并用千分位格式；可选 highlight 按某一列的数值条件把命中单元格文字标红（如 {column: 2, op: \">\", value: 100}）。需要公式、复杂样式、合并单元格、图表、数据校验或修改已有 xlsx 时，改用 script_run 写脚本（exceljs 或 openpyxl）。执行前会请求用户确认，成功后会覆盖原文件。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": ".xlsx 文件绝对路径" },
                "sheets": {
                    "type": "array",
                    "description": "工作表列表，最多 32 张",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "工作表名" },
                            "rows": {
                                "type": "array",
                                "description": "二维数组，行→列；第一行视为表头（加粗、冻结）；单元格值按 JSON 类型写入",
                                "items": { "type": "array", "items": {} }
                            },
                            "highlight": {
                                "type": "object",
                                "description": "可选：按数值条件把某一列命中的单元格文字标红（表头行不参与）",
                                "properties": {
                                    "column": { "type": "integer", "description": "列下标，从 0 开始" },
                                    "op": { "type": "string", "enum": [">", "<", "=="], "description": "比较运算符" },
                                    "value": { "type": "number", "description": "比较阈值" }
                                },
                                "required": ["column", "op", "value"]
                            }
                        },
                        "required": ["name", "rows"]
                    }
                }
            },
            "required": ["path", "sheets"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        if args.sheets.is_empty() {
            return Err(AiToolError::Execution("sheets is empty".to_string()));
        }
        if args.sheets.len() > MAX_SHEETS {
            return Err(AiToolError::Execution(format!("sheets {} exceed limit {}", args.sheets.len(), MAX_SHEETS)));
        }
        let path = Self::resolve_path(&args.path)?;
        PathGuard::ensure_writable(&path)?;
        let confirm_args = ToolXlsxWriteArgs { path: args.path.clone(), sheets: args.sheets.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let sheets = args.sheets.clone();
        let save_path = path.clone();
        let _guard = self.tool_manager.lock_path(&ToolManager::local_path_key(&path)).await;
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            if let Some(parent) = save_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
            }
            let mut wb = ToolXlsxWrite::build_workbook(&sheets)?;
            wb.save(&save_path).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| AiToolError::Execution(error.to_string()))?
        .map_err(AiToolError::Execution)?;
        drop(_guard);
        let body = format!("wrote {} sheet(s) to {}", args.sheets.len(), path.display());
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
