use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use sea_orm::JsonValue;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::model::job_indicator::JobIndicator;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::model::dto::job::indicator_arg::IndicatorArg;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_job_device::JobDeviceService;
use crate::model::req::req_job_indicator::ReqJobIndicator;
use crate::model::job_device_2_indicator::JobDevice2Indicator;
use crate::service::service_job_indicator::JobIndicatorService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::model::dto::job::indicator_arg_template::IndicatorArgType;
use crate::model::job_indicator::{JobIndicatorAlertRuleOp, JobIndicatorDisplayUnit, JobIndicatorParserType, JobIndicatorProtocol};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolJobIndicatorUpdateArgs {
    pub device_id: String,
    pub indicator_id: String,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub command: Option<String>,

    #[serde(default)]
    pub protocol: Option<String>,

    #[serde(default)]
    pub parser: Option<String>,

    #[serde(default)]
    pub parse_expr: Option<String>,

    #[serde(default)]
    pub interval_secs: Option<i64>,

    #[serde(default)]
    pub alert_op: Option<String>,

    #[serde(default)]
    pub alert_threshold: Option<String>,

    #[serde(default)]
    pub unit: Option<String>,

    #[serde(default)]
    pub display_unit: Option<String>,

    #[serde(default)]
    pub transform_expr: Option<String>,

    #[serde(default)]
    pub range_min: Option<f64>,

    #[serde(default)]
    pub range_max: Option<f64>,

    #[serde(default)]
    pub enabled: Option<bool>,

    #[serde(default)]
    pub alert_enabled: Option<bool>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolJobIndicatorUpdate {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    job_device_service: JobDeviceService,

    #[inject(component)]
    job_indicator_service: JobIndicatorService,
}

#[async_trait]
impl BaseApp for ToolJobIndicatorUpdate {
    fn name(&self) -> &'static str {
        "ToolJobIndicatorUpdate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolJobIndicatorUpdate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolJobIndicatorUpdate
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolJobIndicatorUpdate {
    const NAME: &'static str = AiToolId::ToolJobIndicatorUpdate.as_str();

    type Args = ToolJobIndicatorUpdateArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "修改这台设备上某条已生效的正式指标，存下即生效。必须先用 job_indicator_list 拿到 indicator_id 和现有取值，不要凭猜改。只传要改的字段，没传的一律保持原值。改动取值命令或解析表达式时要先在设备上实测跑通新写法；改阈值要基于实测基线。enabled 控制是否继续采集，alert_enabled 控制是否判阈值告警。目标指标 is_default 为 true 时是多台设备共用的内置指标，改它会影响其它设备，动之前先跟用户确认。执行前会请求用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "设备 id，取自用户的 @[device] 引用或 select_all_device 的返回，不是设备名"
                },
                "indicator_id": {
                    "type": "string",
                    "description": "要修改的指标 id，取自 job_indicator_list 返回的 indicator_id"
                },
                "name": { "type": "string", "description": "改指标名称，语言遵循 [输出语言]。不改就不传" },
                "command": { "type": "string", "description": "改取值命令。必须只读、非交互、能自行结束、开销小。不改就不传" },
                "protocol": { "type": "string", "enum": ["ssh", "telnet", "icmp", "snmp"], "description": "改采集协议。不改就不传。改成 snmp 时 command 必须同步改成 JSON 采集参数（op/oids/reduce/rate）" },
                "parser": { "type": "string", "enum": ["exit_code", "regex", "json_path"], "description": "改解析方式。不改就不传" },
                "parse_expr": { "type": "string", "description": "改解析表达式。parser=regex 时必须带一个捕获组。不改就不传" },
                "unit": { "type": "string", "description": "改取值单位。不改就不传" },
                "transform_expr": { "type": "string", "description": "改值转换脚本，rhai 语法，变量 value 是原始字符串，返回值必须是数字。传空字符串表示去掉转换。不改就不传" },
                "display_unit": { "type": "string", "enum": ["none", "bytes", "bitrate", "millis", "seconds", "count"], "description": "改界面显示换算方式，传 none 表示不换算。不改就不传" },
                "range_min": { "type": "number", "description": "改取值的物理下限（原值），用来锚定图表纵轴，例如百分比类指标填 0。不改就不传" },
                "range_max": { "type": "number", "description": "改取值的物理上限（原值），例如百分比类指标填 100。不改就不传" },
                "interval_secs": { "type": "integer", "description": "改采集周期（秒）。不改就不传" },
                "alert_op": { "type": "string", "enum": ["gt", "gte", "lt", "lte", "eq"], "description": "改告警比较方式。不改就不传" },
                "alert_threshold": { "type": "string", "description": "改告警阈值。不改就不传" },
                "enabled": { "type": "boolean", "description": "改是否继续采集。不改就不传" },
                "alert_enabled": { "type": "boolean", "description": "改是否判阈值告警。不改就不传" }
            },
            "required": ["device_id", "indicator_id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let device_id = args.device_id.trim().to_string();
        if device_id.is_empty() {
            return Err(AiToolError::Execution("device_id must not be empty".to_string()));
        }
        let indicator_id = args.indicator_id.trim().to_string();
        if indicator_id.is_empty() {
            return Err(AiToolError::Execution("indicator_id must not be empty".to_string()));
        }
        let (job_device_id, link) = self.resolve_link(&device_id, &indicator_id).await?;
        let Some(current) = link.job_indicator.as_ref() else {
            return Err(AiToolError::Execution(format!("definition of indicator {} is missing", indicator_id)));
        };
        let indicator = Self::apply_patch(current, &args)?;
        let interval_secs = args.interval_secs.unwrap_or(link.interval_secs);

        let mut confirm_args = args.clone();
        confirm_args.internal_call_id = None;
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;

        let name = indicator.name.clone();
        let req = ReqJobIndicator { interval_secs, job_device_id, job_indicator: indicator };
        self.job_indicator_service.save_with_job(req).await.map_err(|error| AiToolError::Execution(format!("failed to save indicator \"{}\": {:#}", name, error)))?;
        let body = format!("已修改设备 {} 上的指标「{}」（indicator_id={}），采集周期 {} 秒，改动已生效。", device_id, name, indicator_id, interval_secs);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolJobIndicatorUpdate {
    async fn resolve_link(&self, device_id: &str, indicator_id: &str) -> Result<(String, JobDevice2Indicator), AiToolError> {
        let job_devices = self.job_device_service.list_detail().await.map_err(|error| AiToolError::Execution(format!("failed to query inspection devices: {:#}", error)))?;
        let Some(job_device) = job_devices.into_iter().find(|item| item.device_id == device_id) else {
            return Err(AiToolError::Execution(format!("device {} is not in inspection yet and has no indicators to modify", device_id)));
        };
        let job_device_id = job_device.id.clone();
        let links = job_device.indicators.unwrap_or_default();
        let Some(link) = links.into_iter().find(|item| item.job_indicator_id == indicator_id) else {
            return Err(AiToolError::Execution(format!("device {} has no indicator with indicator_id={}; check with job_indicator_list first", device_id, indicator_id)));
        };
        Ok((job_device_id, link))
    }

    fn apply_patch(current: &JobIndicator, args: &ToolJobIndicatorUpdateArgs) -> Result<JobIndicator, AiToolError> {
        let mut indicator = current.clone();
        if let Some(name) = args.name.as_ref() {
            indicator.name = name.clone();
        }
        if let Some(protocol) = args.protocol.as_deref() {
            indicator.protocol = JobIndicatorProtocol::from_name(protocol).ok_or_else(|| AiToolError::Execution(format!("unknown collection protocol: {}", protocol)))? as i16;
        }
        if let Some(parser) = args.parser.as_deref() {
            indicator.parser_type = JobIndicatorParserType::from_name(parser).ok_or_else(|| AiToolError::Execution(format!("unknown parser type: {}", parser)))? as i16;
        }
        if let Some(parse_expr) = args.parse_expr.as_ref() {
            indicator.parse_expr = parse_expr.clone();
        }
        if let Some(alert_op) = args.alert_op.as_deref() {
            indicator.alert_rule_op = JobIndicatorAlertRuleOp::from_name(alert_op).ok_or_else(|| AiToolError::Execution(format!("unknown alert comparison operator: {}", alert_op)))? as i16;
        }
        if let Some(alert_threshold) = args.alert_threshold.as_ref() {
            indicator.alert_threshold = alert_threshold.clone();
        }
        if let Some(display_unit) = args.display_unit.as_deref() {
            indicator.display_unit = Self::display_unit_code(display_unit)?;
        }
        if let Some(transform_expr) = args.transform_expr.as_ref() {
            indicator.transform_expr = transform_expr.clone();
        }
        if let Some(range_min) = args.range_min {
            indicator.range_min = Some(range_min);
        }
        if let Some(range_max) = args.range_max {
            indicator.range_max = Some(range_max);
        }
        if let Some(enabled) = args.enabled {
            indicator.enabled = enabled;
        }
        if let Some(alert_enabled) = args.alert_enabled {
            indicator.alert_enabled = alert_enabled;
        }
        indicator.arg = Self::patch_arg(current, &indicator, args)?;
        Ok(indicator)
    }

    fn patch_arg(current: &JobIndicator, patched: &JobIndicator, args: &ToolJobIndicatorUpdateArgs) -> Result<JsonValue, AiToolError> {
        let mut arg = Self::resolve_arg(current)?;
        if let Some(command) = args.command.as_ref() {
            arg.value = command.clone();
        }
        if let Some(unit) = args.unit.as_ref() {
            arg.unit = unit.clone();
        }
        arg.template.protocol = patched.protocol;
        arg.template.arg_type = if patched.protocol == JobIndicatorProtocol::Icmp as i16 {
            IndicatorArgType::Number as i16
        } else if patched.protocol == JobIndicatorProtocol::Snmp as i16 {
            IndicatorArgType::Snmp as i16
        } else {
            IndicatorArgType::Shell as i16
        };
        serde_json::to_value(&arg).map_err(|error| AiToolError::Execution(format!("failed to serialize indicator arguments: {}", error)))
    }

    fn resolve_arg(indicator: &JobIndicator) -> Result<IndicatorArg, AiToolError> {
        let arg = indicator.get_arg().map_err(|error| AiToolError::Execution(format!("failed to parse arguments of indicator {}: {:#}", indicator.name, error)))?;
        arg.ok_or_else(|| AiToolError::Execution(format!("indicator {} has no collection arguments", indicator.name)))
    }

    fn display_unit_code(value: &str) -> Result<i16, AiToolError> {
        if value == JobIndicatorDisplayUnit::None.as_str() {
            return Ok(JobIndicatorDisplayUnit::None as i16);
        }
        let display_unit = JobIndicatorDisplayUnit::from_name(value).ok_or_else(|| AiToolError::Execution(format!("unknown display unit: {}", value)))?;
        Ok(display_unit as i16)
    }
}
