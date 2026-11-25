use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::collections::HashMap;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::group::DEFAULT_GROUP_ID;
use crate::model::resp::resp_page::QueryPage;
use crate::service::service_base::BaseService;
use crate::module::ai::shared::ToolCallDonePush;
use crate::service::service_group::GroupService;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::service::service_device::DeviceService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::model::resp::resp_ai_group_info::RespAiGroupInfo;
use crate::model::resp::resp_ai_devices_info::RespAiDevicesInfo;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

fn default_page_size() -> usize {
    5
}

const PAGE_SIZE_MAX: usize = 20;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolGroupArgs {
    #[serde(default)]
    pub page_number: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default)]
    pub keyword: Option<String>,
}

#[base_app(sort = 763)]
#[derive(Clone, Service)]
pub struct ToolGroup {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    group_service: GroupService,

    #[inject(component)]
    device_service: DeviceService,
}

#[async_trait]
impl BaseApp for ToolGroup {
    fn name(&self) -> &'static str {
        "ToolGroup"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolGroup {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolGroupList
    }

    fn dedup_in_run(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolGroup {
    fn parse_tokens(keyword: Option<&str>) -> Vec<String> {
        match keyword {
            Some(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    trimmed.split_whitespace().map(|token| token.to_string()).collect()
                }
            }
            None => Vec::new(),
        }
    }

    fn score_group(group: &RespAiGroupInfo, tokens: &[String]) -> usize {
        let group_name = group.name.to_lowercase();
        tokens
            .iter()
            .filter(|token| {
                if group.id.as_str() == token.as_str() {
                    return true;
                }
                let lower = token.to_lowercase();
                if group_name.contains(lower.as_str()) {
                    return true;
                }
                group.devices.iter().any(|device| device.id.as_str() == token.as_str() || device.name.to_lowercase().contains(lower.as_str()))
            })
            .count()
    }

    fn filter_groups(groups: Vec<RespAiGroupInfo>, tokens: &[String]) -> Vec<RespAiGroupInfo> {
        let mut scored: Vec<(usize, RespAiGroupInfo)> = groups.into_iter().map(|group| (Self::score_group(&group, tokens), group)).filter(|(score, _)| *score > 0).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase())));
        scored.into_iter().map(|(_, group)| group).collect()
    }
}

impl Tool for ToolGroup {
    const NAME: &'static str = AiToolId::ToolGroupList.as_str();

    type Args = ToolGroupArgs;
    type Output = QueryPage<RespAiGroupInfo>;
    type Error = AiToolError;

    fn description(&self) -> String {
        "查询本应用已登记的设备分组，以及每个分组下的设备（分组 id/名称、设备 id/名称），支持分页。按可选关键字匹配：分组 id 精确匹配；分组名或其成员设备名部分匹配（大小写不敏感）；命中越多越靠前。不传关键字则按分页返回分组。分页字段：total 为全量匹配数（不受分页影响），page_number 从 0 开始，page_size 为每页条数，rows 为当前页数据；total > page_size 时递增 page_number 继续查。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "keyword": {
                    "type": "string",
                    "description": "可选关键字，空格分隔多个词。分组 id 与设备 id 仅精确匹配；分组名与设备名大小写不敏感部分匹配；命中越多越靠前。不传则按分页返回。"
                },
                "page_number": {
                    "type": "integer",
                    "description": "页码，从 0 开始，默认 0。",
                    "default": 0
                },
                "page_size": {
                    "type": "integer",
                    "description": "每页返回条数，默认 5，上限 20。total 大于 page_size 时需翻页查询。",
                    "default": 5
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let groups = self.group_service.find_all().await.map_err(|err| AiToolError::Execution(err.to_string()))?;
        let devices = self.device_service.select_all_with_config().await.map_err(|err| AiToolError::Execution(err.to_string()))?;

        let mut devices_by_group: HashMap<String, Vec<RespAiDevicesInfo>> = HashMap::new();
        for item in devices {
            let group_id = match item.group_id() {
                Some(id) => id.to_string(),
                None => DEFAULT_GROUP_ID.to_string(),
            };
            let device = item.device();
            devices_by_group.entry(group_id).or_default().push(RespAiDevicesInfo { id: device.id.clone(), name: device.name.clone() });
        }

        let all: Vec<RespAiGroupInfo> = groups
            .into_iter()
            .map(|group_ptr| {
                let group = group_ptr.read().clone();
                let devices = devices_by_group.remove(&group.id).unwrap_or_default();
                RespAiGroupInfo { id: group.id, name: group.name, devices }
            })
            .collect();

        let tokens = Self::parse_tokens(args.keyword.as_deref());
        let groups = if tokens.is_empty() { all } else { Self::filter_groups(all, &tokens) };
        let page = QueryPage::new(args.page_number, args.page_size.clamp(1, PAGE_SIZE_MAX), groups);
        let body = serde_json::to_string(&page).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(page)
    }
}
