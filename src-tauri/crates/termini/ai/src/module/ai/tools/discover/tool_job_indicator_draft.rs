use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::model::job_discover_item::JobDiscoverItem;
use crate::model::job_indicator::JobIndicatorProtocol;
use crate::model::dto::job::indicator_arg::IndicatorArg;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_job_discover::JobDiscoverService;
use crate::module::ai::flow::flow_job_discover::FlowJobDiscover;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolJobIndicatorDraftItem {
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
    pub reason: Option<String>,

    #[serde(default)]
    pub sample_value: Option<String>,

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
pub struct ToolJobIndicatorDraftArgs {
    pub items: Vec<ToolJobIndicatorDraftItem>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolJobIndicatorDraft {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    flow_job_discover: FlowJobDiscover,

    #[inject(component)]
    job_discover_service: JobDiscoverService,
}

#[async_trait]
impl BaseApp for ToolJobIndicatorDraft {
    fn name(&self) -> &'static str {
        "ToolJobIndicatorDraft"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolJobIndicatorDraft {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolJobIndicatorDraft
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Discover]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolJobIndicatorDraft {
    const NAME: &'static str = AiToolId::ToolJobIndicatorDraft.as_str();

    type Args = ToolJobIndicatorDraftArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "提交本次设备探索发现的候选监控指标，只在指标探索任务里可用，普通对话调用会直接报错。每条候选都必须是你已经在目标设备上实测跑通、并确认解析表达式能提出值的指标；没实测过的不要提交。可以分批多次调用，边探边交。提交后不会立刻开始采集，也不会开始告警，要等运维在界面上确认采纳。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "description": "本批候选指标，至少一条",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "指标名称，语言遵循 [输出语言]，运维一眼能看懂" },
                            "command": { "type": "string", "description": "取值命令。ssh/telnet 填 shell 命令，必须只读、非交互、能自行结束、开销小；icmp 填 ping 次数；snmp 填 JSON 采集参数，如 {\"op\":\"walk\",\"oids\":[\"1.3.6.1.2.1.31.1.1.1.6\"],\"reduce\":\"sum\",\"rate\":true}" },
                            "protocol": { "type": "string", "enum": ["ssh", "telnet", "icmp", "snmp"], "description": "采集协议，按设备实际可用的协议选。用 snmp 前必须先用 snmp_probe 实测过 OID" },
                            "parser": { "type": "string", "enum": ["exit_code", "regex", "json_path"], "description": "解析方式：exit_code 取退出码；regex 用正则的第一个捕获组；json_path 从 JSON 输出里取值。protocol=snmp 时不走解析，固定填 exit_code" },
                            "parse_expr": { "type": "string", "description": "解析表达式。parser=regex 时写正则且必须带一个捕获组；parser=json_path 时写 JSONPath；parser=exit_code 或 protocol=snmp 时留空字符串" },
                            "unit": { "type": "string", "description": "取值本身的单位，例如 % / bytes / ms，没有就不传" },
                            "transform_expr": { "type": "string", "description": "值转换脚本，rhai 语法，变量 value 是解析出来的原始字符串，返回值必须是数字。取值本来就是数字时不要传。例如接口状态 up/down 折算成 1/0：if value == \"up\" { 1 } else { 0 }" },
                            "display_unit": { "type": "string", "enum": ["bytes", "bitrate", "millis", "seconds", "count"], "description": "界面显示时的换算方式，只影响好不好读，存和比的仍是原值。bytes 用于字节数，bitrate 用于 bps 速率，millis/seconds 用于时长，count 用于加千分位的大整数。是百分比或本来就好读的小数字就不要传" },
                            "range_min": { "type": "number", "description": "取值的物理下限（原值），用来锚定图表纵轴，例如百分比类指标填 0。只在能确定时填；无下界或无法评估就不传" },
                            "range_max": { "type": "number", "description": "取值的物理上限（原值），例如百分比类指标填 100。只在能确定时填；无上界（如流量、连接数）或无法评估就不传" },
                            "interval_secs": { "type": "integer", "description": "建议采集周期（秒）。变化快的资源类指标建议 60，变化慢的建议 300 以上" },
                            "alert_op": { "type": "string", "enum": ["gt", "gte", "lt", "lte", "eq"], "description": "告警比较方式，实测值与阈值的关系满足时告警" },
                            "alert_threshold": { "type": "string", "description": "告警阈值，按你实测到的基线定，不要拍脑袋。这只是给运维的建议值，采纳后告警仍是关闭状态，要他自己开" },
                            "sample_value": { "type": "string", "description": "你这次实测拿到的真实值，原样填" },
                            "reason": { "type": "string", "description": "为什么建议监控它，以及阈值是怎么定出来的，一两句话，语言遵循 [输出语言]" }
                        },
                        "required": ["name", "command", "protocol", "parser", "parse_expr", "interval_secs", "alert_op", "alert_threshold"]
                    }
                }
            },
            "required": ["items"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some((discover_id, job_device_id)) = self.flow_job_discover.running_discover(&run_context.req_id) else {
            return Err(AiToolError::Execution("not in an indicator discovery task; candidate indicators cannot be submitted".to_string()));
        };
        if args.items.is_empty() {
            return Err(AiToolError::Execution("this batch has no candidate indicators; submit at least one".to_string()));
        }
        let mut candidates = Vec::with_capacity(args.items.len());
        for item in &args.items {
            candidates.push(Self::build_candidate(item, &discover_id, &job_device_id)?);
        }
        let names: Vec<String> = candidates.iter().map(|item| item.name.clone()).collect();
        let saved = self.job_discover_service.add_candidates(candidates).await.map_err(|error| AiToolError::Execution(format!("failed to save candidate indicators: {:#}", error)))?;
        let body = format!("已提交 {} 条候选指标：{}。等运维在界面上确认采纳。", saved, names.join("、"));
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolJobIndicatorDraft {
    fn build_candidate(item: &ToolJobIndicatorDraftItem, discover_id: &str, job_device_id: &str) -> Result<JobDiscoverItem, AiToolError> {
        let protocol = Self::protocol_code(&item.protocol).ok_or_else(|| AiToolError::Execution(format!("unknown collection protocol: {}", item.protocol)))?;
        let parser_type = Self::parser_code(&item.parser).ok_or_else(|| AiToolError::Execution(format!("unknown parser type: {}", item.parser)))?;
        let alert_rule_op = Self::alert_op_code(&item.alert_op).ok_or_else(|| AiToolError::Execution(format!("unknown alert comparison operator: {}", item.alert_op)))?;
        let display_unit = item.display_unit.as_deref().map(Self::display_unit_code).unwrap_or(0);
        let arg_type = if protocol == JobIndicatorProtocol::Icmp as i16 {
            IndicatorArgType::Number as i16
        } else if protocol == JobIndicatorProtocol::Snmp as i16 {
            IndicatorArgType::Snmp as i16
        } else {
            IndicatorArgType::Shell as i16
        };
        let arg = IndicatorArg {
            unit: item.unit.clone().unwrap_or_default(),
            value: item.command.clone(),
            template: IndicatorArgTemplate { id: String::new(), arg_type, label_i18n: String::new(), protocol },
        };
        let alert_threshold = item.alert_threshold.clone();
        let transform_expr = item.transform_expr.clone().unwrap_or_default();
        let arg = serde_json::to_value(&arg).map_err(|error| AiToolError::Execution(format!("failed to serialize candidate indicator arguments: {}", error)))?;
        Ok(JobDiscoverItem {
            arg,
            status: 0,
            protocol,
            parser_type,
            timestamp: 0,
            display_unit,
            alert_rule_op,
            transform_expr,
            alert_threshold,
            id: String::new(),
            job_indicator_id: None,
            name: item.name.clone(),
            range_min: item.range_min,
            range_max: item.range_max,
            reason: item.reason.clone(),
            interval_secs: item.interval_secs,
            parse_expr: item.parse_expr.clone(),
            discover_id: discover_id.to_string(),
            sample_value: item.sample_value.clone(),
            job_device_id: job_device_id.to_string(),
        })
    }

    fn protocol_code(value: &str) -> Option<i16> {
        JobIndicatorProtocol::from_name(value).map(|protocol| protocol as i16)
    }

    fn parser_code(value: &str) -> Option<i16> {
        match value {
            "exit_code" => Some(1),
            "regex" => Some(2),
            "json_path" => Some(3),
            _ => None,
        }
    }

    fn alert_op_code(value: &str) -> Option<i16> {
        match value {
            "gt" => Some(1),
            "gte" => Some(2),
            "lt" => Some(3),
            "lte" => Some(4),
            "eq" => Some(5),
            _ => None,
        }
    }

    fn display_unit_code(value: &str) -> i16 {
        match value {
            "bytes" => 1,
            "bitrate" => 2,
            "millis" => 3,
            "seconds" => 4,
            "count" => 5,
            _ => 0,
        }
    }
}
