use serde::{Deserialize, Serialize};
use crate::model::dto::job::indicator_arg_template::IndicatorArgTemplate;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorArg {
    pub unit: String,                   // 单位，如 % / ms，可空
    pub value: String,                  // 通用取值：Shell→脚本，Number→数字，后续类型按 arg_type 解释
    pub template: IndicatorArgTemplate, // 模板信息
}
