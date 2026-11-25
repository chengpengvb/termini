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
use crate::web::ws::ws_manager::WsManager;
use crate::service::service_ai_crud::handler;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::service::service_ai_crud::{LIST_LIMIT_DEFAULT, LIST_LIMIT_MAX};
use crate::model::resp::event::resp_event_entity_notify::RespEventEntityNotify;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolEntityCallArgs {
    pub op: String,

    pub entity: String,

    #[serde(default)]
    pub id: Option<String>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub keyword: Option<String>,

    #[serde(default)]
    pub ids: Option<Vec<String>>,

    #[serde(default)]
    pub row: Option<serde_json::Value>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,

    #[serde(default)]
    pub rows: Option<Vec<serde_json::Value>>,
}

#[base_app(sort = 776)]
#[derive(Clone, Service)]
pub struct ToolEntityCall {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolEntityCall {
    fn name(&self) -> &'static str {
        "ToolEntityCall"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolEntityCall {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolEntityCall
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolEntityCall {
    fn require_id(id: Option<String>) -> Result<String, AiToolError> {
        let Some(id) = id else {
            return Err(AiToolError::Execution("id is required".to_string()));
        };
        if id.trim().is_empty() {
            return Err(AiToolError::Execution("id is empty".to_string()));
        }
        Ok(id)
    }

    fn require_row(row: Option<serde_json::Value>) -> Result<serde_json::Value, AiToolError> {
        let Some(row) = row else {
            return Err(AiToolError::Execution("row is required".to_string()));
        };
        if !row.is_object() {
            return Err(AiToolError::Execution("row must be an object".to_string()));
        }
        Ok(row)
    }

    fn require_rows(rows: Option<Vec<serde_json::Value>>) -> Result<Vec<serde_json::Value>, AiToolError> {
        let Some(rows) = rows else {
            return Err(AiToolError::Execution("rows is required".to_string()));
        };
        if rows.is_empty() {
            return Err(AiToolError::Execution("rows is empty".to_string()));
        }
        for row in &rows {
            if !row.is_object() {
                return Err(AiToolError::Execution("row must be an object".to_string()));
            }
        }
        Ok(rows)
    }

    fn require_ids(ids: Option<Vec<String>>) -> Result<Vec<String>, AiToolError> {
        let Some(ids) = ids else {
            return Err(AiToolError::Execution("ids is required".to_string()));
        };
        if ids.is_empty() {
            return Err(AiToolError::Execution("ids is empty".to_string()));
        }
        for id in &ids {
            if id.trim().is_empty() {
                return Err(AiToolError::Execution("id is empty".to_string()));
            }
        }
        Ok(ids)
    }

    fn is_write(op: &str) -> bool {
        matches!(op, "insert" | "update" | "delete")
    }
}

impl Tool for ToolEntityCall {
    const NAME: &'static str = AiToolId::ToolEntityCall.as_str();

    type Error = AiToolError;
    type Args = ToolEntityCallArgs;
    type Output = serde_json::Value;

    fn description(&self) -> String {
        "对 entity_list 列出的全部表做 list/get/insert/update/delete，不限于密钥。要用哪张表必须先 entity_list（可 keyword 过滤）看现有表名，再 entity_describe 看字段，不要只调用 config_key_pair / config_ssh。设备导入、技能、MCP 已有专用工具的走专用工具，不要改走本工具；其余表都走这里。list 用 offset/limit/keyword 翻页，不要一次拉全表。get/update 必须带 id。insert 一条用 row，成批用 rows，不要一条条调。delete 一条用 id，成批用 ids。row 与 rows、id 与 ids 不能同时传。insert/update 的 row 只传要写的字段；hidden 字段读出来是 null，有真实值时写入。update 只改传入字段，其余保持原值。写操作会先请用户确认。导入设备用 device_import，不要用本工具 insert device。SSH 密钥是其中一类：insert config_key_pair（name、private_key，可选 public_key、passphrase、certificate），再 update config_ssh 把 config_key_pair_id 写成返回的 id（用 device_id 找到那一行；每台设备只有一行 config_ssh，不要再 insert）。登录密码写 config_ssh.base.password；改 base 时必须带上原有 port、username，否则会被整段覆盖。写操作成功后前端页面不会自动刷新，须再调 entity_notify，传入对应 entity_type 数字。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "entity": {
                    "type": "string",
                    "description": "表名，必须来自 entity_list 返回的 entity。可读写表有多张，先 list 再选，不要只填 config_key_pair 或 config_ssh"
                },
                "op": {
                    "type": "string",
                    "enum": ["list", "get", "insert", "update", "delete"],
                    "description": "list 分页查行；get 按 id 取一行；insert 新增（row 或 rows）；update 按 id 合并字段；delete 按 id 或 ids 删除"
                },
                "id": {
                    "type": "string",
                    "description": "get/update/delete 单条必填"
                },
                "ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "delete 批量，与 id 互斥"
                },
                "row": {
                    "type": "object",
                    "description": "insert 单条必填；update 必填且必须含 id"
                },
                "rows": {
                    "type": "array",
                    "items": { "type": "object" },
                    "description": "insert 批量，与 row 互斥"
                },
                "keyword": {
                    "type": "string",
                    "description": "list 可选，在行 JSON 上过滤"
                },
                "offset": {
                    "type": "integer",
                    "description": "list 从第几条开始，默认 0"
                },
                "limit": {
                    "type": "integer",
                    "description": "list 本页条数，默认 8，上限 20"
                }
            },
            "required": ["entity", "op"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let op = args.op.trim();
        if Self::is_write(op) {
            let confirm_args = ToolEntityCallArgs { internal_call_id: None, ..args.clone() };
            self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        }
        let crud = handler(&args.entity).map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        let body = match op {
            "list" => {
                let limit = args.limit.unwrap_or(LIST_LIMIT_DEFAULT).clamp(1, LIST_LIMIT_MAX);
                crud.list(args.offset.unwrap_or(0), limit, args.keyword).await
            }
            "get" => crud.get(Self::require_id(args.id)?).await,
            "insert" => match (args.row, args.rows) {
                (Some(row), None) => crud.insert(Self::require_row(Some(row))?).await,
                (None, Some(rows)) => crud.insert_many(Self::require_rows(Some(rows))?).await,
                (None, None) => Err(anyhow::anyhow!("row or rows is required")),
                (Some(_), Some(_)) => Err(anyhow::anyhow!("row and rows cannot both be set")),
            },
            "update" => crud.update(Self::require_row(args.row)?).await,
            "delete" => match (args.id, args.ids) {
                (Some(id), None) => crud.delete(Self::require_id(Some(id))?).await,
                (None, Some(ids)) => crud.delete_many(Self::require_ids(Some(ids))?).await,
                (None, None) => Err(anyhow::anyhow!("id or ids is required")),
                (Some(_), Some(_)) => Err(anyhow::anyhow!("id and ids cannot both be set")),
            },
            other => Err(anyhow::anyhow!("unknown op: {}", other)),
        }
        .map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        if Self::is_write(op) {
            self.ws_manager.broadcast_all(&RespEventEntityNotify::OnChanged(crud.notify_entity_type()).to_resp());
        }
        let rendered = serde_json::to_string(&body).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, rendered)]);
        Ok(body)
    }
}
