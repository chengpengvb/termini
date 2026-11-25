use serde::{Deserialize, Serialize};

pub enum IndicatorArgType {
    Shell = 1,  // Shell 脚本（value 为脚本正文）
    Number = 2, // 数字（value 为数字字符串，如 ICMP 次数）
    Snmp = 3,   // SNMP 采集参数（value 为 JSON：op/oids/reduce/rate）
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorArgTemplate {
    pub id: String,
    pub arg_type: i16,
    pub label_i18n: String, // 国际化标签

    #[serde(default)]
    pub protocol: i16, // 归属协议，前端按它找该用哪个模板
}
