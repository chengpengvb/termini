use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::job_indicator::JobIndicator;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::model::dto::job::indicator_arg::IndicatorArg;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_job_device::JobDeviceService;
use crate::model::job_device_2_indicator::JobDevice2Indicator;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::service::service_ai_crud::{LIST_LIMIT_DEFAULT, LIST_LIMIT_MAX};
use crate::model::job_indicator::{JobIndicatorAlertRuleOp, JobIndicatorDisplayUnit, JobIndicatorParserType, JobIndicatorProtocol};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolJobIndicatorListArgs {
    pub device_id: String,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolJobIndicatorListItem {
    pub indicator_id: String,
    pub name: String,
    pub protocol: String,
    pub command: String,
    pub parser: String,
    pub parse_expr: String,
    pub interval_secs: i64,
    pub enabled: bool,
    pub alert_enabled: bool,
    pub alert_op: String,
    pub alert_threshold: String,
    pub is_default: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_unit: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_expr: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_min: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_max: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolJobIndicatorListOutput {
    pub device_id: String,
    pub in_monitor: bool,
    pub monitor_enabled: bool,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub indicators: Vec<ToolJobIndicatorListItem>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolJobIndicatorList {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    job_device_service: JobDeviceService,
}

#[async_trait]
impl BaseApp for ToolJobIndicatorList {
    fn name(&self) -> &'static str {
        "ToolJobIndicatorList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolJobIndicatorList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolJobIndicatorList
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolJobIndicatorList {
    const NAME: &'static str = AiToolId::ToolJobIndicatorList.as_str();

    type Args = ToolJobIndicatorListArgs;
    type Output = ToolJobIndicatorListOutput;
    type Error = AiToolError;

    fn description(&self) -> String {
        "按 device_id 查这台设备当前生效的监控指标（正式指标，不是候选）。只读。改指标前必须先用它看现状：返回的 indicator_id 是 job_indicator_update 的入参，name / command / alert_threshold 等字段是判断该改哪条、原值是什么的依据。keyword 按指标名过滤；offset/limit 翻页，total 大于本页时继续翻。in_monitor=false 表示还没纳入巡检、一条指标都没有；monitor_enabled=false 表示已纳入但采集被暂停，此时改指标能存下但不会真跑。is_default=true 的是多台设备共用的内置指标，改它会影响其它设备，动之前先跟用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "设备 id，取自用户的 @[device] 引用或 select_all_device 的返回，不是设备名"
                },
                "keyword": {
                    "type": "string",
                    "description": "可选，按指标名过滤"
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
            "required": ["device_id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let device_id = args.device_id.trim().to_string();
        if device_id.is_empty() {
            return Err(AiToolError::Execution("device_id must not be empty".to_string()));
        }
        let limit = args.limit.unwrap_or(LIST_LIMIT_DEFAULT).clamp(1, LIST_LIMIT_MAX);
        let offset = args.offset.unwrap_or(0);
        let job_devices = self.job_device_service.list_detail().await.map_err(|error| AiToolError::Execution(format!("failed to query inspection devices: {:#}", error)))?;
        let output = match job_devices.into_iter().find(|item| item.device_id == device_id) {
            None => ToolJobIndicatorListOutput { device_id, in_monitor: false, monitor_enabled: false, total: 0, offset, limit, indicators: Vec::new() },
            Some(job_device) => {
                let links = job_device.indicators.unwrap_or_default();
                let mut indicators = Vec::with_capacity(links.len());
                for link in &links {
                    indicators.push(Self::build_item(link)?);
                }
                let (total, indicators) = Self::page_indicators(indicators, args.keyword.as_deref(), offset, limit);
                ToolJobIndicatorListOutput { device_id, in_monitor: true, monitor_enabled: job_device.enabled, total, offset, limit, indicators }
            }
        };
        let body = serde_json::to_string(&output).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(output)
    }
}

impl ToolJobIndicatorList {
    fn page_indicators(indicators: Vec<ToolJobIndicatorListItem>, keyword: Option<&str>, offset: usize, limit: usize) -> (usize, Vec<ToolJobIndicatorListItem>) {
        let filtered: Vec<ToolJobIndicatorListItem> = match keyword.map(str::trim).filter(|query| !query.is_empty()) {
            Some(query) => {
                let lower = query.to_lowercase();
                indicators.into_iter().filter(|item| item.name.to_lowercase().contains(&lower)).collect()
            }
            None => indicators,
        };
        let total = filtered.len();
        let rows = filtered.into_iter().skip(offset).take(limit).collect();
        (total, rows)
    }

    fn build_item(link: &JobDevice2Indicator) -> Result<ToolJobIndicatorListItem, AiToolError> {
        let Some(indicator) = link.job_indicator.as_ref() else {
            return Err(AiToolError::Execution(format!("definition of indicator {} is missing", link.job_indicator_id)));
        };
        let arg = Self::resolve_arg(indicator)?;
        let protocol = JobIndicatorProtocol::from_i16(indicator.protocol)
            .ok_or_else(|| AiToolError::Execution(format!("indicator {} has an unrecognized protocol code {}", indicator.name, indicator.protocol)))?;
        let parser = JobIndicatorParserType::from_i16(indicator.parser_type)
            .ok_or_else(|| AiToolError::Execution(format!("indicator {} has an unrecognized parser type code {}", indicator.name, indicator.parser_type)))?;
        let alert_op = JobIndicatorAlertRuleOp::from_i16(indicator.alert_rule_op)
            .ok_or_else(|| AiToolError::Execution(format!("indicator {} has an unrecognized alert operator code {}", indicator.name, indicator.alert_rule_op)))?;
        Ok(ToolJobIndicatorListItem {
            indicator_id: indicator.id.clone(),
            name: indicator.name.clone(),
            protocol: protocol.as_str().to_string(),
            command: arg.value,
            parser: parser.as_str().to_string(),
            parse_expr: indicator.parse_expr.clone(),
            interval_secs: link.interval_secs,
            enabled: indicator.enabled,
            alert_enabled: indicator.alert_enabled,
            alert_op: alert_op.as_str().to_string(),
            alert_threshold: indicator.alert_threshold.clone(),
            is_default: indicator.is_default,
            unit: Self::some_when_filled(arg.unit),
            range_min: indicator.range_min,
            range_max: indicator.range_max,
            display_unit: Self::resolve_display_unit(indicator)?,
            transform_expr: Self::some_when_filled(indicator.transform_expr.clone()),
        })
    }

    fn resolve_arg(indicator: &JobIndicator) -> Result<IndicatorArg, AiToolError> {
        let arg = indicator.get_arg().map_err(|error| AiToolError::Execution(format!("failed to parse arguments of indicator {}: {:#}", indicator.name, error)))?;
        arg.ok_or_else(|| AiToolError::Execution(format!("indicator {} has no collection arguments", indicator.name)))
    }

    fn resolve_display_unit(indicator: &JobIndicator) -> Result<Option<String>, AiToolError> {
        if indicator.display_unit == JobIndicatorDisplayUnit::None as i16 {
            return Ok(None);
        }
        let display_unit = JobIndicatorDisplayUnit::from_i16(indicator.display_unit)
            .ok_or_else(|| AiToolError::Execution(format!("indicator {} has an unrecognized display unit code {}", indicator.name, indicator.display_unit)))?;
        Ok(Some(display_unit.as_str().to_string()))
    }

    fn some_when_filled(value: String) -> Option<String> {
        if value.trim().is_empty() {
            return None;
        }
        Some(value)
    }
}
