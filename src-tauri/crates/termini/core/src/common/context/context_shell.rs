use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::common::context::context_base::ContextBase;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ShellDescr {
    pub shell_id: String,
    pub args: Vec<String>,
    pub description: String, // 终端描述（给 AI / 展示用）
    pub exe: Option<PathBuf>,
    pub supports_osc633: bool,
    pub env: Vec<(String, String)>, //默认环境变量
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConfigShell {
    pub cwd: Option<String>,
    pub shell_descr: ShellDescr,
    pub env: Vec<(String, String)>, //额外环境变量 TODO (系统配置)
    pub cpu_percent: i16,           //本次执行的 CPU 上限百分比，0 或 100 表示不限制；用户自己开的终端一律不限制
}

pub type ContextShell = ContextBase<ConfigShell>;
