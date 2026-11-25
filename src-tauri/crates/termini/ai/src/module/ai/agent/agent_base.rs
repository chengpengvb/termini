use anyhow::Result;
use crate::module::ai::dto::AiRunEmit;
use crate::module::ai::provide::ai_agent::AiAgent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum AgentRole {
    Router = 1,   //分类
    Main = 2,     //主Agent
    Audit = 3,    //工具执行前的操作审核
    Alert = 4,    //告警诊断
    Discover = 5, //指标探索发现
    Direct = 8,   // 问答
}

impl AgentRole {
    pub fn as_name(self) -> String {
        match self {
            AgentRole::Router => "分流".to_string(),
            AgentRole::Main => "助手".to_string(),
            AgentRole::Audit => "审核".to_string(),
            AgentRole::Alert => "告警诊断".to_string(),
            AgentRole::Discover => "指标探索".to_string(),
            AgentRole::Direct => "问答".to_string(),
        }
    }
}

pub trait AgentBase: Send + Sync {
    fn role(&self) -> AgentRole;

    fn max_turns(&self) -> usize;

    fn emit(&self) -> AiRunEmit;

    fn tool_concurrency(&self) -> usize;

    fn build_ai_agent(&self, req_id: &str) -> Result<AiAgent>;
}
