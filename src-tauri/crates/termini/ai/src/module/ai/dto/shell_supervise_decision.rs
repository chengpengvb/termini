use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellSuperviseAction {
    Wait = 1,   // 继续等：进程还在推进，按 wait_seconds 睡一觉再看
    Kill = 2,   // 终止：卡在等输入或已经没有意义，杀掉并把原因带回调用方
    Detach = 3, // 转交：进程继续跑，工具立刻返回调用方；转异步还是挂载由调用方的声明决定
}

impl ShellSuperviseAction {
    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellSuperviseDecision {
    pub reason: String,
    pub wait_seconds: u64,
    #[serde(default)]
    pub wake_patterns: Vec<String>,
    pub action: ShellSuperviseAction,
}

impl ShellSuperviseDecision {
    pub fn match_wake(&self, text: &str) -> Option<String> {
        if self.wake_patterns.is_empty() || text.is_empty() {
            return None;
        }
        let lower = text.to_lowercase();
        self.wake_patterns.iter().find(|pattern| !pattern.trim().is_empty() && lower.contains(&pattern.to_lowercase())).cloned()
    }
}
