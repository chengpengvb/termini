use rig::tool::DynamicTool;
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::tools::base::TOOL_OUTPUT_MAX_TOKENS;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum ToolDomain {
    Main,     // 主对话(用户交互)
    Alert,    // 告警处理(后台)
    Discover, // 指标探索(后台)
    Audit,    // 只给审核代理用, 主代理看不到(主代理有功能更全的同类工具)
}

impl ToolDomain {
    pub const MAIN_READ_ONLY: &'static [Self] = &[Self::Main, Self::Audit];
    pub const ALL: &'static [Self] = &[Self::Main, Self::Alert, Self::Discover];
    pub const READ_ONLY: &'static [Self] = &[Self::Main, Self::Alert, Self::Discover, Self::Audit];
}

pub trait ToolBase: Send + Sync {
    fn tool_id(&self) -> AiToolId;

    fn on_stop(&self, _req_id: &str) {}

    fn on_start(&self, _req_id: &str) {}

    fn dedup_in_run(&self) -> bool {
        false
    }

    fn supersede_in_run(&self) -> bool {
        false
    }

    fn persist(&self) -> bool {
        false
    }

    fn read_only(&self) -> bool {
        false
    }

    fn deferred(&self) -> bool {
        false
    }

    fn platform_supported(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::ALL
    }

    fn max_result_tokens(&self) -> usize {
        TOOL_OUTPUT_MAX_TOKENS
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool;

    fn as_gated_tool(&self) -> DynamicTool {
        self.as_rig_tool(None)
    }
}
