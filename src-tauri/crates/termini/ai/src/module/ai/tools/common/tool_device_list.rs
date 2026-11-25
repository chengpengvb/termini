use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::resp::resp_page::QueryPage;
use crate::model::resp::resp_device::RespDevice;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::service::service_device::DeviceService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

fn default_page_size() -> usize {
    5
}

const PAGE_SIZE_MAX: usize = 20;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolDeviceListArgs {
    #[serde(default)]
    pub page_number: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default)]
    pub keyword: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolDeviceList {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    device_service: DeviceService,
}

#[async_trait]
impl BaseApp for ToolDeviceList {
    fn name(&self) -> &'static str {
        "ToolDeviceList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDeviceList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDeviceList
    }

    fn dedup_in_run(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolDeviceList {
    const NAME: &'static str = AiToolId::ToolDeviceList.as_str();

    type Args = ToolDeviceListArgs;
    type Output = QueryPage<RespDevice>;
    type Error = AiToolError;

    fn description(&self) -> String {
        "查询本应用已登记的设备列表（连接注册表），支持分页。关键字：设备 id 精确匹配，设备名或 host 模糊匹配（大小写不敏感）；不传则返回全部。只能查已登记设备，不能发现或操作操作系统内容。列出的都是须按 device_id 用远程执行工具操作的独立目标，名称与 host 仅是标签，不得据此判定为宿主机或改用本地执行工具。config_ssh / config_telnet 表示该设备配置了哪些协议：有 config_ssh 用 ssh_shell_exec_batch，只有 config_telnet（常见于不支持 SSH 的交换机、路由器）用 telnet_shell_exec_batch（一台也走批量工具，device_ids 传一个）。password、private_key、passphrase、certificate 等凭据字段始终为 null，这不代表未配置，凭据由系统在连接时自动注入，不要据此向用户索要密码。device.profile 是已沉淀的设备档案（角色、关键服务与路径、运维约定、已知坑）：有值就先读它再动手，不要重复探索档案里已有的结论；发现档案与实际不符、或获得新的长期认知时，用 device_profile_update 在旧档案基础上合并提炼后写回。分页字段：total 为全量匹配数（不受分页影响），page_number 从 0 开始，page_size 为每页条数，rows 为当前页数据；total > page_size 时递增 page_number 继续查。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "keyword": {
                    "type": "string",
                    "description": "可选关键字，空格分隔多个词。设备 id 仅精确匹配；设备名与 host 大小写不敏感部分匹配；命中越多越靠前。不传则返回全部。"
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
        let devices = self.device_service.select_all_with_config().await.map_err(|err| AiToolError::Execution(err.to_string()))?;

        let tokens: Vec<String> = match args.keyword.as_deref() {
            Some(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    trimmed.split_whitespace().map(|token| token.to_string()).collect()
                }
            }
            None => Vec::new(),
        };

        let filtered: Vec<RespDevice> = if tokens.is_empty() {
            devices.into_iter().map(|d| d.redact()).collect()
        } else {
            let mut scored: Vec<(usize, RespDevice)> = devices
                .into_iter()
                .map(|item| {
                    let device = item.device();
                    let name = device.name.to_lowercase();
                    let host = device.host.to_lowercase();
                    let score = tokens
                        .iter()
                        .filter(|token| {
                            if device.id.as_str() == token.as_str() {
                                return true;
                            }
                            let lower = token.to_lowercase();
                            name.contains(lower.as_str()) || host.contains(lower.as_str())
                        })
                        .count();
                    (score, item)
                })
                .filter(|(score, _)| *score > 0)
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.device().name.to_lowercase().cmp(&b.1.device().name.to_lowercase())));
            scored.into_iter().map(|(_, item)| item.redact()).collect()
        };

        let page = QueryPage::new(args.page_number, args.page_size.clamp(1, PAGE_SIZE_MAX), filtered);
        let body = serde_json::to_string(&page).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(page)
    }
}
