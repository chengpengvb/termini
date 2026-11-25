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
use crate::model::job_indicator::JobIndicator;
use crate::service::service_base::BaseService;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::model::dto::job::indicator_arg::IndicatorArg;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_job_device::JobDeviceService;
use crate::model::req::req_job_indicator::ReqJobIndicator;
use crate::service::service_job_indicator::JobIndicatorService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use termini_collect::collect::collector::r#impl::collector_snmp::SnmpIndicatorSpec;
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};
use crate::model::job_indicator::{JobIndicatorAlertAiOn, JobIndicatorAlertRuleOp, JobIndicatorDisplayUnit, JobIndicatorParserType, JobIndicatorProtocol};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolJobIndicatorCreateItem {
    pub name: String,
    pub command: String,
    pub protocol: String,
    pub parser: String,
    pub parse_expr: String,
    pub interval_secs: i64,
    pub alert_op: String,
    pub alert_threshold: String,

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
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolJobIndicatorCreateArgs {
    pub device_id: String,
    pub items: Vec<ToolJobIndicatorCreateItem>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolJobIndicatorCreate {
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
impl BaseApp for ToolJobIndicatorCreate {
    fn name(&self) -> &'static str {
        "ToolJobIndicatorCreate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolJobIndicatorCreate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolJobIndicatorCreate
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolJobIndicatorCreate {
    const NAME: &'static str = AiToolId::ToolJobIndicatorCreate.as_str();

    type Args = ToolJobIndicatorCreateArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "给指定设备新增正式监控指标，存下即生效、下一个周期就开始采集。每条都必须是你已经在这台设备上实测跑通、并确认解析表达式能提出值的指标；没实测过的不要提交。取值命令必须只读、非交互、能自行结束、开销小——它之后每个周期都会重复执行。阈值按实测基线定，不要拍脑袋。新增后告警默认关闭，需要用户自己开。可以一次提交多条。设备还没纳入巡检时会自动为它建巡检记录。执行前会请求用户确认。这个工具写的是正式指标，与指标探索任务里提交候选、等人工采纳是两回事。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "设备 id，取自用户的 @[device] 引用或 select_all_device 的返回，不是设备名"
                },
                "items": {
                    "type": "array",
                    "description": "本批要新增的指标，至少一条",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "指标名称，语言遵循 [输出语言]，运维一眼能看懂" },
                            "command": { "type": "string", "description": "取值命令。ssh/telnet 填 shell 命令，必须只读、非交互、能自行结束、开销小；icmp 填 ping 次数；snmp 填 JSON 采集参数，如 {\"op\":\"walk\",\"oids\":[\"1.3.6.1.2.1.31.1.1.1.6\"],\"reduce\":\"sum\",\"rate\":true}，op 支持 get/walk，reduce 支持 first/sum/avg/max/min/count，rate=true 表示按两轮差值算每秒速率（Counter 类必填）" },
                            "protocol": { "type": "string", "enum": ["ssh", "telnet", "icmp", "snmp"], "description": "采集协议，按设备实际可用的协议选。用 snmp 前必须先用 snmp_probe 实测过 OID" },
                            "parser": { "type": "string", "enum": ["exit_code", "regex", "json_path"], "description": "解析方式：exit_code 取退出码；regex 用正则的第一个捕获组；json_path 从 JSON 输出里取值。protocol=snmp 时不走解析，固定填 exit_code" },
                            "parse_expr": { "type": "string", "description": "解析表达式。parser=regex 时写正则且必须带一个捕获组；parser=json_path 时写 JSONPath；parser=exit_code 或 protocol=snmp 时留空字符串" },
                            "unit": { "type": "string", "description": "取值本身的单位，例如 % / bytes / ms，没有就不传" },
                            "transform_expr": { "type": "string", "description": "值转换脚本，rhai 语法，变量 value 是解析出来的原始字符串，返回值必须是数字。取值本来就是数字时不要传。例如接口状态 up/down 折算成 1/0：if value == \"up\" { 1 } else { 0 }" },
                            "display_unit": { "type": "string", "enum": ["bytes", "bitrate", "millis", "seconds", "count"], "description": "界面显示时的换算方式，只影响好不好读，存和比的仍是原值。bytes 用于字节数，bitrate 用于 bps 速率，millis/seconds 用于时长，count 用于加千分位的大整数。是百分比或本来就好读的小数字就不要传" },
                            "range_min": { "type": "number", "description": "取值的物理下限（原值），用来锚定图表纵轴，例如百分比类指标填 0。只在能确定时填；无下界或无法评估就不传" },
                            "range_max": { "type": "number", "description": "取值的物理上限（原值），例如百分比类指标填 100。只在能确定时填；无上界（如流量、连接数）或无法评估就不传" },
                            "interval_secs": { "type": "integer", "description": "采集周期（秒）。变化快的资源类指标建议 60，变化慢的建议 300 以上" },
                            "alert_op": { "type": "string", "enum": ["gt", "gte", "lt", "lte", "eq"], "description": "告警比较方式，实测值与阈值的关系满足时告警" },
                            "alert_threshold": { "type": "string", "description": "告警阈值，按你实测到的基线定。新增后告警仍是关闭状态，要用户自己开" }
                        },
                        "required": ["name", "command", "protocol", "parser", "parse_expr", "interval_secs", "alert_op", "alert_threshold"]
                    }
                }
            },
            "required": ["device_id", "items"]
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
        if args.items.is_empty() {
            return Err(AiToolError::Execution("this batch has no indicators to add; submit at least one".to_string()));
        }
        let mut indicators = Vec::with_capacity(args.items.len());
        for item in &args.items {
            indicators.push(Self::build_indicator(item)?);
        }

        let confirm_args = ToolJobIndicatorCreateArgs { device_id: device_id.clone(), items: args.items.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;

        let job_device_id = self.resolve_job_device_id(&device_id).await?;
        let mut created = Vec::with_capacity(indicators.len());
        for (item, indicator) in args.items.iter().zip(indicators) {
            let req = ReqJobIndicator { interval_secs: item.interval_secs, job_device_id: job_device_id.clone(), job_indicator: indicator };
            let link = self.job_indicator_service.save_with_job(req).await.map_err(|error| AiToolError::Execution(format!("failed to save indicator \"{}\": {:#}", item.name, error)))?;
            created.push(format!("{}（indicator_id={}）", item.name, link.job_indicator_id));
        }
        let body = format!("已为设备 {} 新增 {} 条指标并开始采集：{}。告警默认关闭，需要的话让用户在界面上开。", device_id, created.len(), created.join("、"));
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolJobIndicatorCreate {
    async fn resolve_job_device_id(&self, device_id: &str) -> Result<String, AiToolError> {
        if let Some(job_device_id) = self.find_job_device_id(device_id).await? {
            return Ok(job_device_id);
        }
        self.job_device_service.save_batch(vec![device_id.to_string()]).await.map_err(|error| AiToolError::Execution(format!("failed to add device {} to inspection: {:#}", device_id, error)))?;
        match self.find_job_device_id(device_id).await? {
            Some(job_device_id) => Ok(job_device_id),
            None => Err(AiToolError::Execution(format!("device {} does not exist and cannot be added to inspection", device_id))),
        }
    }

    async fn find_job_device_id(&self, device_id: &str) -> Result<Option<String>, AiToolError> {
        let job_devices = self.job_device_service.find_all().await.map_err(|error| AiToolError::Execution(format!("failed to query inspection devices: {:#}", error)))?;
        Ok(job_devices.iter().find(|item| item.read().device_id == device_id).map(|item| item.read().id.clone()))
    }

    fn build_indicator(item: &ToolJobIndicatorCreateItem) -> Result<JobIndicator, AiToolError> {
        let protocol = JobIndicatorProtocol::from_name(&item.protocol).ok_or_else(|| AiToolError::Execution(format!("unknown collection protocol: {}", item.protocol)))? as i16;
        let parser_type = JobIndicatorParserType::from_name(&item.parser).ok_or_else(|| AiToolError::Execution(format!("unknown parser type: {}", item.parser)))? as i16;
        let alert_rule_op = JobIndicatorAlertRuleOp::from_name(&item.alert_op).ok_or_else(|| AiToolError::Execution(format!("unknown alert comparison operator: {}", item.alert_op)))? as i16;
        let display_unit = match item.display_unit.as_deref() {
            Some(value) => JobIndicatorDisplayUnit::from_name(value).ok_or_else(|| AiToolError::Execution(format!("unknown display unit: {}", value)))? as i16,
            None => JobIndicatorDisplayUnit::None as i16,
        };
        let arg_type = if protocol == JobIndicatorProtocol::Icmp as i16 {
            IndicatorArgType::Number as i16
        } else if protocol == JobIndicatorProtocol::Snmp as i16 {
            serde_json::from_str::<SnmpIndicatorSpec>(&item.command)
                .map_err(|error| AiToolError::Execution(format!("SNMP collection parameters of indicator \"{}\" are not valid JSON (requires op/oids, optional reduce/rate): {}", item.name, error)))?;
            IndicatorArgType::Snmp as i16
        } else {
            IndicatorArgType::Shell as i16
        };
        let arg = IndicatorArg {
            unit: item.unit.clone().unwrap_or_default(),
            value: item.command.clone(),
            template: IndicatorArgTemplate { id: String::new(), arg_type, label_i18n: String::new(), protocol },
        };
        let arg = serde_json::to_value(&arg).map_err(|error| AiToolError::Execution(format!("failed to serialize indicator arguments: {}", error)))?;
        Ok(JobIndicator {
            arg,
            sort: 0,
            protocol,
            parser_type,
            display_unit,
            alert_rule_op,
            enabled: true,
            is_default: false,
            id: String::new(),
            alert_enabled: false,
            alert_sensitivity: 0,
            name: item.name.clone(),
            alert_ai_repair_message: None,
            parse_expr: item.parse_expr.clone(),
            range_min: item.range_min,
            range_max: item.range_max,
            alert_ai_on: JobIndicatorAlertAiOn::Off as i16,
            alert_threshold: item.alert_threshold.clone(),
            transform_expr: item.transform_expr.clone().unwrap_or_default(),
        })
    }
}
