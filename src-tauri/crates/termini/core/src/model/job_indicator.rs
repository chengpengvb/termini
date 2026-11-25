use sea_orm::JsonValue;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use crate::model::dto::job::indicator_arg::IndicatorArg;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum JobIndicatorProtocol {
    SSH = 1,     // SSH 协议
    Telnet = 2,  // Telnet 协议
    Icmp = 3,    // ICMP 协议
    Snmp = 4,    // SNMP 协议
    Http = 5,    // HTTP(S) 存活探测
    Tcp = 6,     // TCP 端口探测
    TlsCert = 7, // TLS 证书剩余天数探测
}

impl JobIndicatorProtocol {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(JobIndicatorProtocol::SSH),
            2 => Some(JobIndicatorProtocol::Telnet),
            3 => Some(JobIndicatorProtocol::Icmp),
            4 => Some(JobIndicatorProtocol::Snmp),
            5 => Some(JobIndicatorProtocol::Http),
            6 => Some(JobIndicatorProtocol::Tcp),
            7 => Some(JobIndicatorProtocol::TlsCert),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ssh" => Some(JobIndicatorProtocol::SSH),
            "telnet" => Some(JobIndicatorProtocol::Telnet),
            "icmp" => Some(JobIndicatorProtocol::Icmp),
            "snmp" => Some(JobIndicatorProtocol::Snmp),
            "http" => Some(JobIndicatorProtocol::Http),
            "tcp" => Some(JobIndicatorProtocol::Tcp),
            "tls_cert" => Some(JobIndicatorProtocol::TlsCert),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            JobIndicatorProtocol::SSH => "ssh",
            JobIndicatorProtocol::Telnet => "telnet",
            JobIndicatorProtocol::Icmp => "icmp",
            JobIndicatorProtocol::Snmp => "snmp",
            JobIndicatorProtocol::Http => "http",
            JobIndicatorProtocol::Tcp => "tcp",
            JobIndicatorProtocol::TlsCert => "tls_cert",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum JobIndicatorParserType {
    ExitCode = 1, // 退出码解析器
    Regex = 2,    // 正则解析器
    JsonPath = 3, // JSONPath 解析器
}

impl JobIndicatorParserType {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(JobIndicatorParserType::ExitCode),
            2 => Some(JobIndicatorParserType::Regex),
            3 => Some(JobIndicatorParserType::JsonPath),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "exit_code" => Some(JobIndicatorParserType::ExitCode),
            "regex" => Some(JobIndicatorParserType::Regex),
            "json_path" => Some(JobIndicatorParserType::JsonPath),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            JobIndicatorParserType::ExitCode => "exit_code",
            JobIndicatorParserType::Regex => "regex",
            JobIndicatorParserType::JsonPath => "json_path",
        }
    }
}

/// 只影响前端怎么把数字念给人听，后端存的、比的一律是基础单位的原值
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum JobIndicatorDisplayUnit {
    None = 0,    // 不换算，原样显示
    Bytes = 1,   // 字节 B/KB/MB/GB/TB
    Bitrate = 2, // 速率 bps/Kbps/Mbps/Gbps
    Millis = 3,  // 时长（毫秒）ms/s/min/h
    Seconds = 4, // 时长（秒）s/min/h/d
    Count = 5,   // 大数字千分位
}

impl JobIndicatorDisplayUnit {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(JobIndicatorDisplayUnit::Bytes),
            2 => Some(JobIndicatorDisplayUnit::Bitrate),
            3 => Some(JobIndicatorDisplayUnit::Millis),
            4 => Some(JobIndicatorDisplayUnit::Seconds),
            5 => Some(JobIndicatorDisplayUnit::Count),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "bytes" => Some(JobIndicatorDisplayUnit::Bytes),
            "bitrate" => Some(JobIndicatorDisplayUnit::Bitrate),
            "millis" => Some(JobIndicatorDisplayUnit::Millis),
            "seconds" => Some(JobIndicatorDisplayUnit::Seconds),
            "count" => Some(JobIndicatorDisplayUnit::Count),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            JobIndicatorDisplayUnit::None => "none",
            JobIndicatorDisplayUnit::Bytes => "bytes",
            JobIndicatorDisplayUnit::Bitrate => "bitrate",
            JobIndicatorDisplayUnit::Millis => "millis",
            JobIndicatorDisplayUnit::Seconds => "seconds",
            JobIndicatorDisplayUnit::Count => "count",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum JobIndicatorAlertRuleOp {
    Gt = 1,  // 大于
    Gte = 2, // 大于等于
    Lt = 3,  // 小于
    Lte = 4, // 小于等于
    Eq = 5,  // 等于
}

impl JobIndicatorAlertRuleOp {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(JobIndicatorAlertRuleOp::Gt),
            2 => Some(JobIndicatorAlertRuleOp::Gte),
            3 => Some(JobIndicatorAlertRuleOp::Lt),
            4 => Some(JobIndicatorAlertRuleOp::Lte),
            5 => Some(JobIndicatorAlertRuleOp::Eq),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "gt" => Some(JobIndicatorAlertRuleOp::Gt),
            "gte" => Some(JobIndicatorAlertRuleOp::Gte),
            "lt" => Some(JobIndicatorAlertRuleOp::Lt),
            "lte" => Some(JobIndicatorAlertRuleOp::Lte),
            "eq" => Some(JobIndicatorAlertRuleOp::Eq),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            JobIndicatorAlertRuleOp::Gt => "gt",
            JobIndicatorAlertRuleOp::Gte => "gte",
            JobIndicatorAlertRuleOp::Lt => "lt",
            JobIndicatorAlertRuleOp::Lte => "lte",
            JobIndicatorAlertRuleOp::Eq => "eq",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum JobIndicatorAlertAiOn {
    Off = 0,           // 关闭
    Analyze = 1,       // 仅分析 L1
    RepairConfirm = 2, // 修复确认 L2
}

impl JobIndicatorAlertAiOn {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            x if x == Self::Off as i16 => None,
            x if x == Self::Analyze as i16 => Some(Self::Analyze),
            x if x == Self::RepairConfirm as i16 => Some(Self::RepairConfirm),
            _ => None,
        }
    }
}

#[base_target(table = "job_indicator", entity_type = 10)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub name: String,                            // 指标名称
    pub sort: i16,                               // 前端显示顺序
    pub enabled: bool,                           // 采集 开启 / 关闭
    pub protocol: i16,                           // 协议类型
    pub arg: JsonValue,                          // JSON: IndicatorArg
    pub is_default: bool,                        // 内置默认指标，可被多台设备共用，删设备时不连带删除
    pub parser_type: i16,                        // 解析器类型
    pub parse_expr: String,                      // 解析表达式
    pub display_unit: i16,                       // 前端显示单位换算 JobIndicatorDisplayUnit
    pub alert_enabled: bool,                     // 告警 开启 / 关闭，默认关闭；关掉后不再判阈值，未恢复的单子一并收尾
    pub alert_ai_on: i16,                        // 告警 AI 升级 JobIndicatorAlertAiOn
    pub alert_rule_op: i16,                      // 告警规则
    pub alert_threshold: String,                 // 告警阈值
    pub transform_expr: String,                  // 值转换脚本（rhai），把 up/down 这类非数字取值折算成数字，空表示不转换
    pub alert_sensitivity: i32,                  // 告警敏感度（连续命中次数）
    pub alert_ai_repair_message: Option<String>, // 修复确认 L2 时给 AI 的修复说明；仅 RepairConfirm 注入，None/空表示无额外说明
    pub range_min: Option<f64>, // 取值范围下限（原值），图表纵轴锚定用，None 表示无下界
    pub range_max: Option<f64>, // 取值范围上限（原值），图表纵轴锚定用，None 表示无上界
}

impl Model {
    pub fn get_arg(&self) -> anyhow::Result<Option<IndicatorArg>> {
        if self.arg.is_null() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_value(self.arg.clone())?))
    }
}

pub type JobIndicator = Model;
