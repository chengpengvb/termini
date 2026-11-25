use regex::Regex;
use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use summer::plugin::LazyComponent;
use summer::tracing::{info, warn};
use crate::module::ai::dto::AiToolId;
use crate::util::time_util::now_millis;
use tokio_util::sync::CancellationToken;
use crate::module::ai::agent_chat::AgentChat;
use crate::base::base_app::{AppContext, BaseApp};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::resp::resp_agent::RespAgentShellDaemonRow;
use crate::model::req::req_chat_context::{ChatOrigin, ReqChatContext};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeSourceKind {
    Shell,
    Loop,
}

impl WakeSourceKind {
    pub fn label(self) -> &'static str {
        match self {
            WakeSourceKind::Shell => "后台命令",
            WakeSourceKind::Loop => "循环监控",
        }
    }

    pub fn stop_tool(self) -> &'static str {
        match self {
            WakeSourceKind::Shell => AiToolId::ToolShellProcessKill.as_str(),
            WakeSourceKind::Loop => AiToolId::ToolLoopStop.as_str(),
        }
    }
}

#[derive(Clone)]
pub struct WakeSource {
    pub title: String,
    pub chat_id: String,
    pub source_id: String,
    pub started_ms: i64,
    pub kind: WakeSourceKind,
    pub stop: CancellationToken,
    pub pattern: Option<Regex>,
    pub context: Arc<ReqChatContext>,
}

impl WakeSource {
    pub fn new(kind: WakeSourceKind, source_id: String, title: String, pattern: Option<Regex>, context: Arc<ReqChatContext>) -> Self {
        Self { kind, title, pattern, source_id, chat_id: context.ai_chat_id.clone(), context, started_ms: now_millis(), stop: CancellationToken::new() }
    }

    pub fn elapsed_secs(&self) -> i64 {
        now_millis().saturating_sub(self.started_ms) / 1000
    }

    pub fn to_daemon_row(&self) -> RespAgentShellDaemonRow {
        RespAgentShellDaemonRow {
            tty: false,
            pid: None,
            cwd: None,
            shell_id: None,
            output: String::new(),
            started_ms: self.started_ms,
            command: self.title.clone(),
            process_id: self.source_id.clone(),
        }
    }

    fn summary_text(&self) -> String {
        format!("- {}「{}」 id={} 已运行={}s 停止用 {}", self.kind.label(), self.title, self.source_id, self.elapsed_secs(), self.kind.stop_tool())
    }
}

#[derive(Clone)]
pub struct WakeSignal {
    pub at_ms: i64,
    pub text: String,
    pub title: String,
    pub source_id: String,
    pub kind: WakeSourceKind,
}

impl WakeSignal {
    fn render(&self, index: usize) -> String {
        let at = chrono::DateTime::from_timestamp_millis(self.at_ms).expect("wake signal timestamp out of range").with_timezone(&chrono::Local);
        format!("{}. {} {}「{}」 id={}\n{}", index, at.format("%H:%M:%S"), self.kind.label(), self.title, self.source_id, self.text)
    }
}

pub struct WakePending {
    signals: Vec<WakeSignal>,
    context: Arc<ReqChatContext>,
}

fn create_sources() -> Arc<DashMap<String, DashMap<String, WakeSource>>> {
    Arc::new(DashMap::new())
}

fn create_pending() -> Arc<DashMap<String, WakePending>> {
    Arc::new(DashMap::new())
}

fn create_waking() -> Arc<DashMap<String, ()>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct WakeBus {
    agent_chat: LazyComponent<AgentChat>,

    #[inject(func = create_waking())]
    waking: Arc<DashMap<String, ()>>,

    #[inject(func = create_pending())]
    pending: Arc<DashMap<String, WakePending>>,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(func = create_sources())]
    sources: Arc<DashMap<String, DashMap<String, WakeSource>>>,
}

#[async_trait]
impl BaseApp for WakeBus {
    fn name(&self) -> &'static str {
        "WakeBus"
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        for by_source in self.sources.iter() {
            for source in by_source.value().iter() {
                source.value().stop.cancel();
            }
        }
        self.sources.clear();
        self.pending.clear();
        Ok(())
    }
}

impl WakeBus {
    pub fn register(&self, source: WakeSource) {
        info!("[ai] 登记唤醒源 chat_id={} kind={:?} id={} title={}", source.chat_id, source.kind, source.source_id, source.title);
        let by_source = self.sources.entry(source.chat_id.clone()).or_default();
        by_source.insert(source.source_id.clone(), source);
    }

    pub fn get(&self, chat_id: &str, source_id: &str) -> Option<WakeSource> {
        let by_source = self.sources.get(chat_id)?;
        by_source.get(source_id).map(|source| source.value().clone())
    }

    pub fn list(&self, chat_id: &str) -> Vec<WakeSource> {
        let Some(by_source) = self.sources.get(chat_id) else {
            return Vec::new();
        };
        let mut rows: Vec<WakeSource> = by_source.iter().map(|source| source.value().clone()).collect();
        rows.sort_by_key(|source| source.started_ms);
        rows
    }

    pub fn list_kind(&self, chat_id: &str, kind: WakeSourceKind) -> Vec<WakeSource> {
        self.list(chat_id).into_iter().filter(|source| source.kind == kind).collect()
    }

    pub fn stop(&self, chat_id: &str, source_id: &str) -> Option<WakeSource> {
        let source = self.remove(chat_id, source_id)?;
        source.stop.cancel();
        info!("[ai] 停止并注销唤醒源 chat_id={} kind={:?} id={}", chat_id, source.kind, source_id);
        Some(source)
    }

    pub fn signal(&self, chat_id: &str, source_id: &str, text: String) {
        let Some(source) = self.get(chat_id, source_id) else {
            info!("[ai] 唤醒源已注销, 丢弃信号 chat_id={} id={}", chat_id, source_id);
            return;
        };
        self.push(&source, text);
        self.try_wake(chat_id);
    }

    pub fn finish(&self, chat_id: &str, source_id: &str, text: String) {
        let Some(source) = self.remove(chat_id, source_id) else {
            info!("[ai] 唤醒源已注销, 丢弃结束信号 chat_id={} id={}", chat_id, source_id);
            return;
        };
        self.push(&source, text);
        self.try_wake(chat_id);
    }

    pub fn on_round_end(&self, chat_id: &str) {
        self.try_wake(chat_id);
    }

    pub fn release(&self, chat_id: &str) {
        self.pending.remove(chat_id);
        let Some((_, by_source)) = self.sources.remove(chat_id) else {
            return;
        };
        for source in by_source.iter() {
            source.value().stop.cancel();
        }
    }

    fn remove(&self, chat_id: &str, source_id: &str) -> Option<WakeSource> {
        let by_source = self.sources.get(chat_id)?;
        by_source.remove(source_id).map(|(_, source)| source)
    }

    fn push(&self, source: &WakeSource, text: String) {
        let signal = WakeSignal { at_ms: now_millis(), text, kind: source.kind, title: source.title.clone(), source_id: source.source_id.clone() };
        let mut pending = self.pending.entry(source.chat_id.clone()).or_insert_with(|| WakePending { signals: Vec::new(), context: source.context.clone() });
        pending.signals.push(signal);
    }

    fn try_wake(&self, chat_id: &str) {
        if self.waking.insert(chat_id.to_string(), ()).is_some() {
            return;
        }
        if self.chat_session_cache_manager.get_req_id_by_chat_id(chat_id).is_some() {
            self.waking.remove(chat_id);
            return;
        }
        let Some((_, pending)) = self.pending.remove(chat_id) else {
            self.waking.remove(chat_id);
            return;
        };
        let prompt = self.render_prompt(chat_id, &pending.signals);
        let bus = self.clone();
        let chat_id = chat_id.to_string();
        tokio::spawn(async move {
            info!("[ai] 唤醒新一轮 chat_id={} signals={}", chat_id, pending.signals.len());
            if let Err(error) = bus.run_round(&chat_id, prompt, &pending.context).await {
                warn!("[ai] 唤醒轮次失败 chat_id={} error={:#}", chat_id, error);
            }
            bus.waking.remove(&chat_id);
            bus.try_wake(&chat_id);
        });
    }

    fn render_prompt(&self, chat_id: &str, signals: &[WakeSignal]) -> String {
        let mut text = format!(
            "[后台唤醒] 本对话的后台唤醒源投递了 {} 条信号，按时间先后列出。结合对话记忆判断并继续处理：该做的后续步骤直接做；目标已达成、不再需要某个源时用它对应的停止工具停掉，否则它会继续发信号；处理完没有别的事就收尾说明结论。\n",
            signals.len()
        );
        for (index, signal) in signals.iter().enumerate() {
            text.push_str(&signal.render(index + 1));
            text.push('\n');
        }
        let running = self.list(chat_id);
        if running.is_empty() {
            text.push_str("当前没有仍在运行的后台唤醒源。");
            return text;
        }
        text.push_str("仍在运行的后台唤醒源：\n");
        for source in running {
            text.push_str(&source.summary_text());
            text.push('\n');
        }
        text
    }

    async fn run_round(&self, chat_id: &str, prompt: String, context: &ReqChatContext) -> anyhow::Result<()> {
        let agent_chat = self.agent_chat.get().map_err(|error| anyhow!(error.to_string()))?;
        let context = Arc::new(ReqChatContext {
            message: prompt,
            images: Vec::new(),
            origin: ChatOrigin::Wake,
            ai_chat_id: chat_id.to_string(),
            confirm_mode: context.confirm_mode,
            thinking_level: context.thinking_level.clone(),
            ai_provide_id: context.ai_provide_id.clone(),
        });
        agent_chat.chat_with_context(uuid(), context).await
    }
}
