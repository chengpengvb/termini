use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "command_allow", entity_type = 27)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub tool_id: String,         // 工具名，例如 "ssh_shell_exec"
    pub device_id: String,       // 目标设备 id；无设备工具用 ConflictSubject 的 __local__ 占位，禁止空串
    pub trusted_at: i64,         // 用户点击「记住允许」的时间（毫秒）
    pub command_hash: String,    // 规范化参数的 sha256 十六进制，命令再长也只占固定长度
    pub command_preview: String, // 截断后的命令原文，仅用于展示，匹配一律走 command_hash
}

pub type CommandAllow = Model;

pub const COMMAND_PREVIEW_MAX_LEN: usize = 200;

impl Model {
    pub fn build(device_id: String, tool_id: String, command_hash: String, command_preview: String, trusted_at: i64) -> Self {
        Self { id: String::new(), device_id, tool_id, command_hash, command_preview, trusted_at }
    }
}
