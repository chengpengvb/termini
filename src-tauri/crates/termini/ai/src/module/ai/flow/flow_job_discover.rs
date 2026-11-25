use dashmap::DashMap;
use parking_lot::Mutex;
use tokio::sync::Semaphore;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rig::completion::Message;
use termini_macros::base_app;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use crate::dao::dao_base::BaseDao;
use summer::tracing::{info, warn};
use crate::module::ai::dto::AiToolId;
use crate::dao::dao_device::DeviceDao;
use crate::model::ai_chat::AiChatType;
use crate::model::device::DeviceOsType;
use crate::util::time_util::now_millis;
use tokio_util::sync::CancellationToken;
use crate::dao::dao_job_device::JobDeviceDao;
use crate::module::ai::shared::AiTaskSession;
use crate::model::resp::resp_model::ResultCode;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::dao::dao_job_indicator::JobIndicatorDao;
use crate::service::service_ai_chat::AiChatService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::service::service_config_ssh::ConfigSshService;
use crate::service::service_config_snmp::ConfigSnmpService;
use crate::module::ai::agent::agent_discover::AgentDiscover;
use crate::service::service_job_discover::JobDiscoverService;
use rig::message::{AssistantContent, ToolResult, UserContent};
use crate::service::service_config_telnet::ConfigTelnetService;
use crate::model::job_discover::{JobDiscover, JobDiscoverStatus};
use crate::module::ai::stream::stream_runner::StreamErrorEmitted;
use crate::service::service_job_device_2_indicator::JobDevice2IndicatorService;
use crate::model::req::req_chat_context::{ChatOrigin, ConfirmMode, ReqChatContext};
use crate::module::ai::agent::prompts::{output_language_fixed_text, render, LEAD_DISCOVER_USER};

struct DiscoverRun {
    discover_id: String,
    job_device_id: String,
    req_id: Mutex<Option<String>>,
    cancel_token: CancellationToken,
}

fn create_runs() -> Arc<DashMap<String, DiscoverRun>> {
    Arc::new(DashMap::new())
}

fn create_req_index() -> Arc<DashMap<String, String>> {
    Arc::new(DashMap::new())
}

fn create_gate() -> Arc<OnceLock<Arc<Semaphore>>> {
    Arc::new(OnceLock::new())
}

#[base_app(sort = 770)]
#[derive(Clone, Service)]
pub struct FlowJobDiscover {
    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    agent_discover: AgentDiscover,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    job_device_dao: JobDeviceDao,

    #[inject(component)]
    ai_chat_service: AiChatService,

    #[inject(component)]
    ai_task_session: AiTaskSession,

    #[inject(func = create_gate())]
    gate: Arc<OnceLock<Arc<Semaphore>>>,

    #[inject(component)]
    job_indicator_dao: JobIndicatorDao,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

    #[inject(func = create_runs())]
    runs: Arc<DashMap<String, DiscoverRun>>,

    #[inject(func = create_req_index())]
    req_index: Arc<DashMap<String, String>>,

    #[inject(component)]
    job_discover_service: JobDiscoverService,

    #[inject(component)]
    config_snmp_service: ConfigSnmpService,

    #[inject(component)]
    config_telnet_service: ConfigTelnetService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(component)]
    job_device_2_indicator_service: JobDevice2IndicatorService,
}

#[async_trait]
impl BaseApp for FlowJobDiscover {
    fn name(&self) -> &'static str {
        "FlowJobDiscover"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        let parallelism = match self.config_service.select_config().await {
            Some(config) => config.resolve_job_discover_parallelism(),
            None => 1,
        };
        let _ = self.gate.set(Arc::new(Semaphore::new(parallelism)));
        info!("[ai] 探索并发上限 {}", parallelism);
        Ok(())
    }
}

impl FlowJobDiscover {
    pub async fn start_discover(&self, job_device_id: String) -> Result<JobDiscover> {
        let device_name = self.resolve_device_name(&job_device_id).await?;
        if self.runs.iter().any(|entry| entry.value().job_device_id == job_device_id) {
            return Err(ResultCode::DiscoverInProgress.into());
        }
        let Some(ai_provide_id) = self.resolve_ai_provide_id().await else {
            return Err(ResultCode::DiscoverModelNotSet.into());
        };

        let title = format!("指标探索 · {}", device_name);
        let chat = self.ai_chat_service.start_task(AiChatType::Discover, title).await?;
        let discover = self.job_discover_service.create_run(chat.id.clone(), job_device_id.clone(), Some(ai_provide_id.clone())).await?;

        let cancel_token = CancellationToken::new();
        self.runs.insert(chat.id.clone(), DiscoverRun { discover_id: discover.id.clone(), job_device_id: job_device_id.clone(), req_id: Mutex::new(None), cancel_token: cancel_token.clone() });

        let flow = self.clone();
        let chat_id = chat.id.clone();
        tokio::spawn(async move {
            flow.run_discover(chat_id, job_device_id, ai_provide_id, cancel_token).await;
        });
        Ok(discover)
    }

    pub fn cancel(&self, chat_id: &str) -> Result<()> {
        let Some(run) = self.runs.get(chat_id) else {
            return Err(ResultCode::DiscoverEnded.into());
        };
        let req_id = run.req_id.lock().clone();
        run.cancel_token.cancel();
        drop(run);
        if let Some(req_id) = req_id {
            let _ = self.chat_session_cache_manager.cancel(&req_id);
        }
        Ok(())
    }

    pub fn running_discover(&self, req_id: &str) -> Option<(String, String)> {
        let chat_id = self.req_index.get(req_id).map(|entry| entry.value().clone())?;
        let run = self.runs.get(&chat_id)?;
        Some((run.discover_id.clone(), run.job_device_id.clone()))
    }

    async fn run_discover(&self, chat_id: String, job_device_id: String, ai_provide_id: String, cancel_token: CancellationToken) {
        let result = self.execute(&chat_id, &job_device_id, ai_provide_id, &cancel_token).await;
        let status = if cancel_token.is_cancelled() {
            JobDiscoverStatus::Cancelled
        } else if result.is_err() {
            JobDiscoverStatus::Failed
        } else {
            JobDiscoverStatus::Success
        };
        let summary = result.as_ref().ok().cloned().map(|text| text.trim().to_string()).filter(|text| !text.is_empty());
        let error = match &result {
            Err(error) if status == JobDiscoverStatus::Failed => Some(format!("{:#}", error)),
            _ => None,
        };
        if let Err(error) = &result {
            if status == JobDiscoverStatus::Failed {
                warn!("[ai] 探索结束于失败 chat_id={} error={:#}", chat_id, error);
            }
        }
        if let Err(error) = self.job_discover_service.mark_result(&chat_id, status, summary.clone(), error).await {
            warn!("[ai] 回写探索状态失败 chat_id={} error={:#}", chat_id, error);
        }
        if status == JobDiscoverStatus::Success {
            if let Some(profile) = summary {
                if let Err(error) = self.write_profile(&job_device_id, profile).await {
                    warn!("[ai] 档案写回失败 job_device_id={} error={:#}", job_device_id, error);
                }
            }
        }
        self.runs.remove(&chat_id);
    }

    async fn write_profile(&self, job_device_id: &str, profile: String) -> Result<()> {
        let Some(job_device) = self.job_device_dao.find_by_id(job_device_id.to_string()).await? else {
            return Ok(());
        };
        let device_id = job_device.read().device_id.clone();
        let Some(shared) = self.device_dao.find_by_id(device_id).await? else {
            return Ok(());
        };
        let mut device = shared.read().clone();
        device.profile = Some(profile);
        device.updated_at = now_millis();
        self.device_dao.update(device).await?;
        Ok(())
    }

    async fn execute(&self, chat_id: &str, job_device_id: &str, ai_provide_id: String, cancel_token: &CancellationToken) -> Result<String> {
        let Some(gate) = self.gate.get().cloned() else {
            return Err(ResultCode::DiscoverNotReady.into());
        };
        let permit = tokio::select! {
            permit = gate.acquire_owned() => permit?,
            _ = cancel_token.cancelled() => return Err(ResultCode::DiscoverQueuedCancelled.into()),
        };
        self.job_discover_service.mark_running(chat_id).await?;

        let prompt = self.build_prompt(job_device_id).await?;
        let context = ReqChatContext {
            images: Vec::new(),
            ai_provide_id,
            message: prompt.clone(),
            confirm_mode: ConfirmMode::Silent,
            ai_chat_id: chat_id.to_string(),
            thinking_level: None,
            origin: ChatOrigin::Discover,
        };
        let run = self.ai_task_session.open(context, cancel_token.clone()).await?;
        self.bind_req_id(chat_id, &run.req_id);
        info!("[ai] 开始探索 chat_id={} req_id={}", chat_id, run.req_id);

        let language = self.config_service.select_system_language().await;
        let addon_messages = vec![Message::user(output_language_fixed_text(language.label()))];
        let mut result = self.ai_task_session.run::<AgentDiscover, String>(&run, &self.agent_discover, prompt, addon_messages).await;
        if result.is_ok() {
            if let Some(error) = Self::login_tool_error(&run.memory.lock()) {
                result = Err(anyhow!("{error}"));
            }
        }
        let error = result.as_ref().err().filter(|error| !StreamErrorEmitted::displayed(error)).map(|error| format!("{:#}", error));
        self.req_index.remove(&run.req_id);
        self.ai_task_session.close(run, error).await;
        drop(permit);
        result
    }

    fn bind_req_id(&self, chat_id: &str, req_id: &str) {
        self.req_index.insert(req_id.to_string(), chat_id.to_string());
        let Some(run) = self.runs.get(chat_id) else {
            return;
        };
        *run.req_id.lock() = Some(req_id.to_string());
    }

    async fn resolve_device_name(&self, job_device_id: &str) -> Result<String> {
        let Some(job_device) = self.job_device_dao.find_by_id(job_device_id.to_string()).await? else {
            return Err(ResultCode::DiscoverCollectConfigMissing.into());
        };
        let device_id = job_device.read().device_id.clone();
        let Some(device) = self.device_dao.find_by_id(device_id).await? else {
            return Err(ResultCode::DeviceDeleted.into());
        };
        let name = device.read().name.clone();
        Ok(name)
    }

    async fn resolve_ai_provide_id(&self) -> Option<String> {
        let id = self.config_service.select_config().await?.job_discover_ai_provide_id?;
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_string())
    }

    async fn build_prompt(&self, job_device_id: &str) -> Result<String> {
        let Some(job_device) = self.job_device_dao.find_by_id(job_device_id.to_string()).await? else {
            return Err(ResultCode::DiscoverCollectConfigMissing.into());
        };
        let device_id = job_device.read().device_id.clone();
        let Some(device) = self.device_dao.find_by_id(device_id.clone()).await? else {
            return Err(ResultCode::DeviceDeleted.into());
        };
        let device = device.read().clone();
        let device_profile = device.profile.as_deref().map(str::trim).filter(|text| !text.is_empty()).unwrap_or("（还没有档案，这次总结就是第一版）").to_string();

        Ok(render(
            LEAD_DISCOVER_USER,
            &[
                ("device_id", device_id.as_str()),
                ("device_name", device.name.as_str()),
                ("device_host", device.host.as_str()),
                ("device_os", DeviceOsType::from_i16(device.os_type).label()),
                ("device_protocol", self.protocol_text(&device_id).as_str()),
                ("device_profile", device_profile.as_str()),
                ("exist_indicators", self.exist_indicators(job_device_id).await?.as_str()),
            ],
        ))
    }

    fn protocol_text(&self, device_id: &str) -> String {
        let mut protocols: Vec<&str> = Vec::new();
        let ssh_port = self.config_ssh_service.get_by_device_id(device_id.to_string()).and_then(|ptr| ptr.read().get_base().ok()).map(|base| base.port).unwrap_or(0);
        if ssh_port > 0 {
            protocols.push("ssh");
        }
        let telnet_port = self.config_telnet_service.get_by_device_id(device_id.to_string()).and_then(|ptr| ptr.read().get_base().ok()).map(|base| base.port).unwrap_or(0);
        if telnet_port > 0 {
            protocols.push("telnet");
        }
        let snmp_enabled = self.config_snmp_service.get_by_device_id(device_id.to_string()).map(|ptr| ptr.read().enabled).unwrap_or(false);
        if snmp_enabled {
            protocols.push("snmp");
        }
        if protocols.is_empty() {
            return "这台设备没有配置任何登录/采集协议（ssh/telnet 端口为 0、snmp 未启用），只能做 icmp 连通性探测".to_string();
        }
        protocols.join(" / ")
    }

    async fn exist_indicators(&self, job_device_id: &str) -> Result<String> {
        let indicator_ids = self.job_device_2_indicator_service.list_indicator_ids_by_job_device_ids(&[job_device_id.to_string()]);
        let mut names: Vec<String> = Vec::with_capacity(indicator_ids.len());
        for indicator_id in indicator_ids {
            let Some(shared) = self.job_indicator_dao.find_by_id(indicator_id).await? else {
                continue;
            };
            names.push(format!("- {}", shared.read().name));
        }
        if names.is_empty() {
            return Ok("（这台设备还没有任何指标）".to_string());
        }
        Ok(names.join("\n"))
    }

    /// 探索至少要有一个通道真实接触到设备：ssh/telnet 登录成功，或 snmp_probe 探测成功。
    /// 全部通道都失败才把探索判为失败；纯 SNMP 设备探索成功不再被 ssh 连接失败拖垮。
    fn login_tool_error(messages: &[Message]) -> Option<String> {
        let mut names = HashMap::new();
        let mut failed = Vec::new();
        let mut succeeded = false;
        for message in messages {
            match message {
                Message::Assistant { content, .. } => {
                    for part in content.iter() {
                        let AssistantContent::ToolCall(call) = part else {
                            continue;
                        };
                        let id = call.wire_call_id().to_string();
                        names.insert(id, call.function.name.clone());
                    }
                }
                Message::User { content } => {
                    for part in content.iter() {
                        let UserContent::ToolResult(result) = part else {
                            continue;
                        };
                        let id = result.wire_call_id().to_string();
                        let Some(name) = names.get(&id).map(String::as_str) else {
                            continue;
                        };
                        if Self::is_probe_tool(name) {
                            if Self::probe_output_succeeded(&Self::tool_result_text(result)) {
                                succeeded = true;
                            }
                            continue;
                        }
                        if !Self::is_login_tool(name) {
                            continue;
                        }
                        let text = Self::tool_result_text(result);
                        if Self::login_output_failed(&text) {
                            failed.push(Self::login_output_message(&text));
                        } else {
                            succeeded = true;
                        }
                    }
                }
                _ => {}
            }
        }
        if succeeded || failed.is_empty() {
            return None;
        }
        failed.into_iter().next()
    }

    fn is_login_tool(name: &str) -> bool {
        matches!(AiToolId::from_name(name), Some(AiToolId::ToolSshShellBatch | AiToolId::ToolTelnetShellBatch))
    }

    fn is_probe_tool(name: &str) -> bool {
        matches!(AiToolId::from_name(name), Some(AiToolId::ToolSnmpProbe))
    }

    fn probe_output_succeeded(text: &str) -> bool {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return false;
        };
        value.get("device_id").and_then(|item| item.as_str()).is_some()
    }

    fn tool_result_text(result: &ToolResult) -> String {
        result.content.iter().filter_map(|item| item.as_text()).collect::<Vec<_>>().join("\n")
    }

    fn login_output_failed(text: &str) -> bool {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            if value.get("ok").and_then(|item| item.as_bool()) == Some(false) {
                return true;
            }
            if let Some(items) = value.as_array() {
                if !items.is_empty() && items.iter().all(|item| item.get("ok").and_then(|flag| flag.as_bool()) == Some(false)) {
                    return true;
                }
            }
        }
        text.contains("Failed to connect ssh") || text.contains("Failed to connect telnet")
    }

    fn login_output_message(text: &str) -> String {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(message) = value.get("message").and_then(|item| item.as_str()) {
                if !message.is_empty() {
                    return message.to_string();
                }
            }
            if let Some(items) = value.as_array() {
                for item in items {
                    if let Some(message) = item.get("message").and_then(|value| value.as_str()) {
                        if !message.is_empty() {
                            return message.to_string();
                        }
                    }
                    if let Some(error) = item.get("error").and_then(|value| value.as_str()) {
                        if !error.is_empty() {
                            return Self::login_output_message(error);
                        }
                    }
                }
            }
        }
        text.to_string()
    }
}
