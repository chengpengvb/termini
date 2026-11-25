use anyhow::Result;
use summer::tracing::warn;
use rig::tool::DynamicTool;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use tokio::sync::mpsc::UnboundedSender;
use crate::model::ai_provide::AiProvide;
use tokio_util::sync::CancellationToken;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::service::service_config::ConfigService;
use crate::service::service_device::DeviceService;
use crate::service::service_ai_provide::AiProvideService;
use crate::module::ai::agent::agent_suggest::AgentSuggest;
use crate::module::ai::agent::prompts::output_language_text;
use crate::model::req::req_command_suggest::ReqCommandSuggest;
use crate::model::device::{Device, DeviceOsType, DEVICE_LOCAL_ID};
use crate::module::ai::tools::base::{ToolBaseOutWrap, TOOL_OUTPUT_MAX_TOKENS};
use crate::module::ai::tools::suggest::tool_suggest_web_search::ToolSuggestWebSearch;
use crate::module::ai::agent::agent_build::{prompt_agent_with_turns, stream_prompt_text_until, StreamControl};

const HISTORY_MAX_ITEMS: usize = 12;
const CURRENT_LINE_MAX_CHARS: usize = 400;
const RECENT_OUTPUT_MAX_CHARS: usize = 4000;
const DEVICE_PROFILE_MAX_CHARS: usize = 2000;

#[base_app(sort = 771)]
#[derive(Clone, Service)]
pub struct FlowCommandSuggest {
    #[inject(component)]
    agent_suggest: AgentSuggest,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    ai_provide_service: AiProvideService,
}

#[async_trait]
impl BaseApp for FlowCommandSuggest {
    fn name(&self) -> &'static str {
        "FlowCommandSuggest"
    }
}

impl FlowCommandSuggest {
    pub async fn suggest(&self, req: &ReqCommandSuggest, provide_id: String, count: usize, cancel: CancellationToken, sink: UnboundedSender<Vec<String>>) -> Result<Vec<String>> {
        self.ensure_not_cancelled(&cancel)?;
        let tools = self.build_tools(req);
        let use_search = !tools.is_empty();
        let provide = self.resolve_provide(provide_id).await?;
        let language = self.config_service.select_system_language().await;
        let prompt = format!("{}\n\n{}", output_language_text(language), self.build_user_prompt(req, count).await);
        let agent = self.agent_suggest.build_ai_agent(&provide, tools)?;
        tokio::select! {
            _ = cancel.cancelled() => Err(ResultCode::Cancelled.into()),
            result = async {
                if use_search {
                    let raw = prompt_agent_with_turns(agent, prompt, AgentSuggest::MAX_TURNS).await?;
                    Ok(AgentSuggest::parse_list(&raw, count))
                } else {
                    stream_suggest_list(agent, prompt, count, &sink).await
                }
            } => result,
        }
    }

    fn has_instruction(instruction: Option<&str>) -> bool {
        instruction.map(str::trim).filter(|text| !text.is_empty()).is_some()
    }

    fn build_tools(&self, req: &ReqCommandSuggest) -> Vec<DynamicTool> {
        if !Self::has_instruction(req.instruction.as_deref()) {
            return Vec::new();
        }
        vec![ToolBaseOutWrap::wrap(ToolSuggestWebSearch, TOOL_OUTPUT_MAX_TOKENS, None)]
    }

    fn ensure_not_cancelled(&self, cancel: &CancellationToken) -> Result<()> {
        if cancel.is_cancelled() {
            return Err(ResultCode::Cancelled.into());
        }
        Ok(())
    }

    async fn resolve_provide(&self, provide_id: String) -> Result<AiProvide> {
        let Some(shared) = self.ai_provide_service.find_by_id(provide_id).await? else {
            return Err(ResultCode::AiModelNotFound.into());
        };
        let provide = shared.read().clone();
        Ok(provide)
    }

    async fn build_user_prompt(&self, req: &ReqCommandSuggest, count: usize) -> String {
        let suggest_count = count.to_string();
        let history = describe_history(&req.history);
        let shell_kind = or_empty(req.shell_kind.as_deref());
        let instruction = or_empty(req.instruction.as_deref());
        let device = self.describe_device(&req.device_id).await;
        let alt_screen = if req.alt_screen { "true" } else { "false" };
        let recent_output = or_empty(Some(&truncate_tail(&req.recent_output, RECENT_OUTPUT_MAX_CHARS)));
        let current_line = or_empty(Some(&truncate_tail(&req.current_line, CURRENT_LINE_MAX_CHARS)));
        let web_search_note = if Self::has_instruction(req.instruction.as_deref()) {
            "已挂载 web_search。由你根据 instruction 原意判断用户是否明确要求联网检索，不限语言。只有明确要求才调用；否则直接给候选。调用时一次传 queries（最多 5 条）并发查完，立刻输出候选 JSON，禁止再搜。"
        } else {
            "未挂载联网工具。禁止检索，直接给候选。"
        };
        AgentSuggest::render_user_prompt(&[
            ("alt_screen", alt_screen),
            ("device", device.as_str()),
            ("history", history.as_str()),
            ("shell_kind", shell_kind.as_str()),
            ("instruction", instruction.as_str()),
            ("current_line", current_line.as_str()),
            ("recent_output", recent_output.as_str()),
            ("suggest_count", suggest_count.as_str()),
            ("terminal_kind", req.terminal_kind.as_str()),
            ("web_search_note", web_search_note),
        ])
    }

    async fn describe_device(&self, device_id: &str) -> String {
        let Some(device) = self.resolve_device(device_id).await else {
            return "（无）".to_string();
        };
        let os_label = DeviceOsType::from_i16(device.os_type).label();
        let mut text = if device.id == DEVICE_LOCAL_ID {
            format!("本应用所在的宿主机（本地终端） 系统类型={}", os_label)
        } else {
            format!("名称={} 主机={} 系统类型={}", device.name, device.host, os_label)
        };
        if let Some(profile) = device.profile.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            text.push_str(if device.id == DEVICE_LOCAL_ID {
                "\n宿主机档案（AI 探索沉淀，含系统角色/关键服务/常用路径）：\n"
            } else {
                "\n设备档案（AI 探索沉淀，含系统角色/关键服务/常用路径）：\n"
            });
            text.push_str(&truncate_head(profile, DEVICE_PROFILE_MAX_CHARS));
        }
        text
    }

    async fn resolve_device(&self, device_id: &str) -> Option<Device> {
        match self.device_service.find_by_id(device_id.to_string()).await {
            Ok(shared) => shared.map(|d| d.read().clone()),
            Err(e) => {
                warn!("[ai] 查询设备失败 device_id={} err={}", device_id, e);
                None
            }
        }
    }
}

async fn stream_suggest_list(agent: AiAgent, prompt: String, count: usize, sink: &UnboundedSender<Vec<String>>) -> Result<Vec<String>> {
    let mut acc = String::new();
    let mut last = 0usize;
    let streamed = stream_prompt_text_until(agent, prompt, |delta| {
        acc.push_str(delta);
        let list = AgentSuggest::extract_list(&acc, count);
        if list.len() > last {
            last = list.len();
            if sink.send(list).is_err() {
                return StreamControl::Stop;
            }
        }
        if last >= count {
            StreamControl::Stop
        } else {
            StreamControl::Continue
        }
    })
    .await?;
    Ok(AgentSuggest::parse_list(&streamed.text, count))
}

fn describe_history(history: &[String]) -> String {
    let items: Vec<&String> = history.iter().filter(|line| !line.trim().is_empty()).rev().take(HISTORY_MAX_ITEMS).collect();
    if items.is_empty() {
        return "（无）".to_string();
    }
    items.into_iter().rev().map(|line| format!("- {}", line.trim())).collect::<Vec<String>>().join("\n")
}

fn or_empty(value: Option<&str>) -> String {
    match value.map(|v| v.trim()).filter(|v| !v.is_empty()) {
        Some(v) => v.to_string(),
        None => "（无）".to_string(),
    }
}

fn truncate_tail(input: &str, max_chars: usize) -> String {
    let count = input.chars().count();
    if count <= max_chars {
        return input.to_string();
    }
    input.chars().skip(count - max_chars).collect()
}

fn truncate_head(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let head: String = input.chars().take(max_chars).collect();
    format!("{head}…")
}
