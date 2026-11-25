use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::base_target::AiFieldMeta;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::service::service_ai_crud::{handler, LIST_LIMIT_DEFAULT, LIST_LIMIT_MAX};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolEntityDescribeArgs {
    pub entity: String,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[base_app(sort = 775)]
#[derive(Clone, Service)]
pub struct ToolEntityDescribe {
    #[inject(component)]
    tool_manager: ToolManager,
}

#[async_trait]
impl BaseApp for ToolEntityDescribe {
    fn name(&self) -> &'static str {
        "ToolEntityDescribe"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolEntityDescribe {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolEntityDescribe
    }

    fn read_only(&self) -> bool {
        true
    }

    fn dedup_in_run(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolEntityDescribe {
    const NAME: &'static str = AiToolId::ToolEntityDescribe.as_str();

    type Args = ToolEntityDescribeArgs;
    type Output = serde_json::Value;
    type Error = AiToolError;

    fn description(&self) -> String {
        "查看一张表的字段名、类型、是否敏感。hidden=true 的字段读出来是 null，insert/update 可以写入真实值。keyword 按字段名过滤；offset/limit 翻页，total 大于本页时继续翻。表名必须来自 entity_list，可读写表不限于密钥。SSH 密钥是其中一类：config_key_pair 的 private_key/passphrase/certificate 为 hidden；登录密码在 config_ssh.base.password；把密钥挂到设备写 config_ssh.config_key_pair_id。读写行用 entity_call。"
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "entity": {
                    "type": "string",
                    "description": "表名，来自 entity_list，如 config"
                },
                "keyword": {
                    "type": "string",
                    "description": "可选，按字段名过滤"
                },
                "offset": {
                    "type": "integer",
                    "description": "从第几条开始，默认 0"
                },
                "limit": {
                    "type": "integer",
                    "description": "本页条数，默认 8，上限 20"
                }
            },
            "required": ["entity"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let crud = handler(&args.entity).map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        let mut fields: Vec<&AiFieldMeta> = crud.fields().iter().collect();
        if let Some(query) = args.keyword.as_deref().map(str::trim).filter(|query| !query.is_empty()) {
            let lower = query.to_lowercase();
            fields.retain(|field| field.name.to_lowercase().contains(&lower));
        }
        let total = fields.len();
        let offset = args.offset.unwrap_or(0);
        let limit = args.limit.unwrap_or(LIST_LIMIT_DEFAULT).clamp(1, LIST_LIMIT_MAX);
        let fields: Vec<&AiFieldMeta> = fields.into_iter().skip(offset).take(limit).collect();
        let page = json!({
            "entity": crud.entity(),
            "total": total,
            "offset": offset,
            "limit": limit,
            "fields": fields,
        });
        let body = serde_json::to_string(&page).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(page)
    }
}
