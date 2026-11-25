use std::sync::Arc;
use parking_lot::Mutex;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use rig::completion::Message;
use termini_macros::base_app;
use rig::message::UserContent;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use crate::util::time_util::now_millis;
use chrono::{Datelike, Local, Timelike};
use tokio_util::sync::CancellationToken;
use crate::model::device::DEVICE_LOCAL_ID;
use crate::module::ai::shared::slim_history;
use crate::service::service_base::BaseService;
use crate::service::service_agent::AgentService;
use crate::service::service_group::GroupService;
use crate::service::service_config::ConfigService;
use crate::service::service_device::DeviceService;
use crate::service::service_ai_chat::AiChatService;
use crate::model::resp::resp_agent::RespAgentStatus;
use crate::module::ai::agent::agent_base::AgentBase;
use crate::module::ai::agent::agent_main::AgentMain;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::agent::agent_direct::AgentDirect;
use crate::module::ai::agent::agent_router::AgentRouter;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::module::ai::stream::stream_output::StreamOutput;
use crate::model::dto::chat_memory::{ChatScene, MemoryTurn};
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::agent::prompts::output_language_fixed_text;
use crate::service::service_impl_system_util::SystemUtilServiceImpl;
use crate::module::ai::shared::chat_item_format::CURRENT_GOAL_PREFIX;
use crate::module::ai::dto::{AiRunContext, RouterDecision, SharedMemory, TaskKind};
use crate::module::ai::flow::chat_reference_inject::{build_referenced_mcp_servers_message, build_referenced_prompt_messages, build_referenced_targets_message};

const ROUTER_HISTORY_MAX: usize = 12;

#[base_app(sort = 769)]
#[derive(Clone, Service)]
pub struct FlowAgentChat {
    #[inject(component)]
    agent_main: AgentMain,

    #[inject(component)]
    agent_router: AgentRouter,

    #[inject(component)]
    agent_direct: AgentDirect,

    #[inject(component)]
    stream_runner: AiStreamRunner,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    group_service: GroupService,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    mcp_client_pool: McpClientPool,

    #[inject(component)]
    ai_chat_service: AiChatService,

    #[inject(component)]
    ai_chat_item_service: AiChatItemService,

    #[inject(component)]
    system_util_service: SystemUtilServiceImpl,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

impl BaseApp for FlowAgentChat {
    fn name(&self) -> &'static str {
        "AgentFlow"
    }
}

impl FlowAgentChat {
    pub async fn run(&self, req_id: String, prompt: String, images: Vec<UserContent>, memory: SharedMemory, cancel_token: CancellationToken) -> Result<()> {
        let mut addon_messages = self.build_addon_messages(&req_id, &prompt, &memory).await?;
        self.chat_session_cache_manager.set_addon_messages(&req_id, addon_messages.clone());
        let result = async {
            let decision = match self.restore_scene(&req_id)? {
                Some(decision) => decision,
                None => {
                    self.emit_status(&req_id);
                    let decision = self.classify(req_id.clone(), prompt.clone(), memory.clone(), cancel_token.clone(), addon_messages.clone()).await?;
                    self.apply_title(&req_id, &decision).await;
                    decision
                }
            };
            if cancel_token.is_cancelled() {
                return Ok(());
            }
            self.chat_session_cache_manager.set_scene(&req_id, ChatScene { task_kind: decision.task_kind.as_i16(), goal: decision.goal.clone(), language: decision.language.clone() });
            self.apply_language(&req_id, &memory, &mut addon_messages, &decision)?;
            self.apply_goal(&req_id, &memory, &decision);
            match decision.task_kind {
                TaskKind::MultiStep => self.run_main(req_id.clone(), prompt, images, memory.clone(), addon_messages.clone(), cancel_token).await,
                TaskKind::DirectAnswer => self.run_direct(prompt, images, req_id.clone(), memory.clone(), addon_messages.clone(), cancel_token).await,
            }
        }
        .await;
        Self::remove_addon_messages(&memory, &addon_messages);
        self.chat_session_cache_manager.set_addon_messages(&req_id, Vec::new());
        result
    }

    fn restore_scene(&self, req_id: &str) -> Result<Option<RouterDecision>> {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return Ok(None);
        };
        if !entry.context.origin.resume_scene() {
            return Ok(None);
        }
        let scene = entry.scene.lock().clone();
        drop(entry);
        let Some(scene) = scene else {
            return Ok(None);
        };
        let Some(language) = scene.language else {
            return Ok(None);
        };
        let task_kind = TaskKind::from_i16(scene.task_kind).ok_or_else(|| anyhow!("unknown scene task_kind: {}", scene.task_kind))?;
        info!("[ai] req_id={} 还原现场 task_kind={:?} goal={} language={}", req_id, task_kind, scene.goal.as_deref().unwrap_or("(none)"), language);
        Ok(Some(RouterDecision { task_kind, goal: scene.goal, title: None, language: Some(language) }))
    }

    fn emit_status(&self, req_id: &str) {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return;
        };
        let ai_chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        let resp = RespEventAgent::OnStatus(RespAgentStatus::stage(req_id, "routing"));
        self.agent_service.handler_agent_event(resp, ai_chat_id, None, None, None);
    }

    fn apply_language(&self, req_id: &str, memory: &SharedMemory, addon_messages: &mut Vec<Message>, decision: &RouterDecision) -> Result<()> {
        let language = decision.language.as_deref().map(str::trim).filter(|text| !text.is_empty()).ok_or_else(|| anyhow!("分流未给出 language req_id={}", req_id))?;
        let language_message = Message::user(output_language_fixed_text(language));
        memory.lock().push(language_message.clone());
        addon_messages.push(language_message);
        self.chat_session_cache_manager.set_addon_messages(req_id, addon_messages.clone());
        Ok(())
    }

    fn apply_goal(&self, req_id: &str, memory: &SharedMemory, decision: &RouterDecision) {
        let goal = decision.goal.as_deref().map(str::trim).filter(|g| !g.is_empty());
        info!("[ai] req_id={} task_kind={:?} goal={} title={}", req_id, decision.task_kind, goal.unwrap_or("(none)"), decision.title.as_deref().unwrap_or("(none)"));
        self.retain_turns(req_id);
        let mut guard = memory.lock();
        guard.retain(|message| !Self::is_current_goal_message(message));
        let Some(goal) = goal else {
            return;
        };
        guard.push(Message::user(format!("{CURRENT_GOAL_PREFIX}{goal}")));
    }

    async fn apply_title(&self, req_id: &str, decision: &RouterDecision) {
        let Some(title) = decision.title.clone() else {
            return;
        };
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return;
        };
        let ai_chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        if let Err(error) = self.ai_chat_service.rename(ai_chat_id.clone(), title).await {
            warn!("[ai] 写入会话标题失败 req_id={} chat_id={} error={:#}", req_id, ai_chat_id, error);
        }
    }

    fn retain_turns(&self, req_id: &str) {
        let turns = self.chat_session_cache_manager.turns(req_id);
        let next: Vec<MemoryTurn> =
            turns.into_iter().map(|turn| MemoryTurn { item_id: turn.item_id, messages: turn.messages.into_iter().filter(|message| !Self::is_current_goal_message(message)).collect() }).collect();
        self.chat_session_cache_manager.set_turns(req_id, next);
    }

    fn is_current_goal_message(message: &Message) -> bool {
        match message {
            Message::User { content } => content.iter().any(|part| match part {
                UserContent::Text(text) => text.text.starts_with(CURRENT_GOAL_PREFIX),
                _ => false,
            }),
            _ => false,
        }
    }

    async fn build_addon_messages(&self, req_id: &str, prompt: &str, memory: &SharedMemory) -> Result<Vec<Message>> {
        let mut addon_messages = Vec::new();
        let system_info = self.system_util_service.system_info().await?;
        let host_text = serde_json::to_string(&system_info)?;
        let mut host_message_text = format!("[Operating System Status]{host_text}");
        host_message_text.push_str(&format!("\n[App Language]{}", self.config_service.select_system_language().await.label()));
        if let Some(profile) = self.device_service.select_local_device().read().profile.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
            host_message_text.push_str(&format!("\n[Host Profile device_id={}]\n{}", DEVICE_LOCAL_ID, profile));
        }
        let host_message = Message::user(host_message_text);
        memory.lock().push(host_message.clone());
        addon_messages.push(host_message);

        let time_message = Message::user(self.build_time_message(req_id).await);
        memory.lock().push(time_message.clone());
        addon_messages.push(time_message);

        if let Some(targets_text) = build_referenced_targets_message(prompt, &self.device_service, &self.group_service).await? {
            let targets_message = Message::user(targets_text);
            memory.lock().push(targets_message.clone());
            addon_messages.push(targets_message);
        }
        if let Some(servers_text) = build_referenced_mcp_servers_message(prompt, &self.mcp_client_pool)? {
            let servers_message = Message::user(servers_text);
            memory.lock().push(servers_message.clone());
            addon_messages.push(servers_message);
        }
        for prompt_message in build_referenced_prompt_messages(prompt, &self.mcp_client_pool).await? {
            memory.lock().push(prompt_message.clone());
            addon_messages.push(prompt_message);
        }
        Ok(addon_messages)
    }

    async fn build_time_message(&self, req_id: &str) -> String {
        let now = Local::now();
        let weekday = match now.weekday() {
            chrono::Weekday::Mon => "星期一",
            chrono::Weekday::Tue => "星期二",
            chrono::Weekday::Wed => "星期三",
            chrono::Weekday::Thu => "星期四",
            chrono::Weekday::Fri => "星期五",
            chrono::Weekday::Sat => "星期六",
            chrono::Weekday::Sun => "星期日",
        };
        let period = Self::resolve_day_period(now.hour());
        let mut text = format!("[Current Time]{} {} (UTC{}) · {} (需要精确时刻时用系统命令查询)", now.format("%Y-%m-%d"), weekday, now.format("%:z"), period);
        if let Some(gap) = self.resolve_last_chat_gap(req_id).await {
            text.push_str(&format!("\n距上次对话: {}", gap));
        }
        text
    }

    fn resolve_day_period(hour: u32) -> &'static str {
        match hour {
            0..=5 => "深夜",
            6..=8 => "清晨",
            9..=11 => "上午",
            12..=13 => "午间",
            14..=17 => "下午",
            18..=22 => "晚间",
            _ => "深夜",
        }
    }

    async fn resolve_last_chat_gap(&self, req_id: &str) -> Option<String> {
        let entry = self.chat_session_cache_manager.get(req_id)?;
        let last_item_id = entry.turns.lock().last().map(|turn| turn.item_id.clone());
        drop(entry);
        let shared = self.ai_chat_item_service.find_by_id(last_item_id?).await.ok()??;
        let last_ms = shared.read().timestamp;
        let gap_ms = now_millis() - last_ms;
        if gap_ms < 5 * 60 * 1000 {
            return None;
        }
        Some(Self::format_gap(gap_ms))
    }

    fn format_gap(gap_ms: i64) -> String {
        let mins = gap_ms / 1000 / 60;
        if mins < 60 {
            return "不到 1 小时".to_string();
        }
        let hours = mins / 60;
        if hours < 24 {
            return format!("约 {} 小时", hours);
        }
        format!("约 {} 天", hours / 24)
    }

    fn remove_addon_messages(memory: &SharedMemory, addon_messages: &[Message]) {
        memory.lock().retain(|message| !addon_messages.iter().any(|addon| addon == message));
    }

    async fn classify(&self, req_id: String, prompt: String, memory: SharedMemory, cancel_token: CancellationToken, addon_messages: Vec<Message>) -> Result<RouterDecision> {
        if cancel_token.is_cancelled() {
            return Ok(RouterDecision { task_kind: TaskKind::MultiStep, goal: None, title: None, language: None });
        }
        let history = slim_history(&memory.lock(), ROUTER_HISTORY_MAX);
        let router_memory: SharedMemory = Arc::new(Mutex::new(history));
        match self.run_agent_stream::<_, RouterDecision>(req_id.clone(), prompt, Vec::new(), &self.agent_router, router_memory, addon_messages, cancel_token).await {
            Ok(mut decision) => {
                if decision.language.is_none() {
                    decision.language = Some(self.config_service.select_system_language().await.label().to_string());
                }
                Ok(decision)
            }
            Err(error) => {
                warn!("[ai] 分流失败, 回退 MultiStep 继续执行 req_id={} err={}", req_id, error);
                Ok(RouterDecision { task_kind: TaskKind::MultiStep, goal: None, title: None, language: Some(self.config_service.select_system_language().await.label().to_string()) })
            }
        }
    }

    async fn run_direct(&self, prompt: String, images: Vec<UserContent>, req_id: String, memory: SharedMemory, addon_messages: Vec<Message>, cancel_token: CancellationToken) -> Result<()> {
        let _: String = self.run_agent_stream(req_id, prompt, images, &self.agent_direct, memory, addon_messages, cancel_token).await?;
        Ok(())
    }

    async fn run_main(&self, req_id: String, prompt: String, images: Vec<UserContent>, memory: SharedMemory, addon_messages: Vec<Message>, cancel_token: CancellationToken) -> Result<()> {
        let _: String = self.run_agent_stream(req_id, prompt, images, &self.agent_main, memory, addon_messages, cancel_token).await?;
        Ok(())
    }

    async fn run_agent_stream<A, O>(&self, req_id: String, prompt: String, images: Vec<UserContent>, agent_base: &A, memory: SharedMemory, addons: Vec<Message>, cancel: CancellationToken) -> Result<O>
    where
        A: AgentBase + Clone + 'static,
        O: StreamOutput,
    {
        let emit = agent_base.emit();
        let role = agent_base.role();
        let builder = agent_base.clone();
        let max_turns = agent_base.max_turns();
        let concurrency = agent_base.tool_concurrency();
        let agent = agent_base.build_ai_agent(&req_id)?;
        let chat_id = self.chat_session_cache_manager.chat_id(&req_id);
        let rebuild_agent = Arc::new(move |id: &str| builder.build_ai_agent(id));
        let run_context = Arc::new(AiRunContext::new(req_id, agent, prompt, emit, max_turns, memory, concurrency, chat_id, rebuild_agent, addons, cancel).with_prompt_images(images));
        self.stream_runner.run_stream::<O>(run_context, role).await
    }
}
