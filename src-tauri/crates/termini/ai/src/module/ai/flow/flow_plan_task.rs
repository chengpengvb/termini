use std::sync::Arc;
use dashmap::DashMap;
use std::time::Duration;
use anyhow::{bail, Result};
use summer::plugin::Service;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::model::ai_chat::AiChatType;
use crate::util::time_util::now_millis;
use crate::web::ws::ws_manager::WsManager;
use crate::model::ai_plan_task::AiPlanTask;
use crate::module::ai::agent_chat::AgentChat;
use crate::service::service_base::BaseService;
use crate::dao::dao_ai_plan_task::AiPlanTaskDao;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_ai_chat::AiChatService;
use crate::service::service_ai_plan_run::AiPlanRunService;
use crate::model::ai_plan_run::{AiPlanRun, AiPlanRunStatus};
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::service::service_ai_plan_task::AiPlanTaskService;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::model::resp::event::resp_event_ai_plan::RespEventAiPlan;
use crate::model::req::req_chat_context::{ChatOrigin, ConfirmMode, ReqChatContext};

const TICK_SECONDS: u64 = 30;
const SUMMARY_MAX_CHARS: usize = 200;

fn create_running_map() -> Arc<DashMap<String, ()>> {
    Arc::new(DashMap::new())
}

fn create_run_req_map() -> Arc<DashMap<String, String>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 775)]
#[derive(Clone, Service)]
pub struct FlowPlanTask {
    #[inject(component)]
    agent_chat: AgentChat,

    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    ai_plan_task_dao: AiPlanTaskDao,

    #[inject(component)]
    ai_chat_service: AiChatService,

    #[inject(component)]
    ai_plan_run_service: AiPlanRunService,

    #[inject(component)]
    ai_plan_task_service: AiPlanTaskService,

    #[inject(component)]
    ai_chat_item_service: AiChatItemService,

    #[inject(func = create_running_map())]
    running: Arc<DashMap<String, ()>>,

    #[inject(func = create_run_req_map())]
    run_req: Arc<DashMap<String, String>>,
}

#[async_trait::async_trait]
impl BaseApp for FlowPlanTask {
    fn name(&self) -> &'static str {
        "FlowPlanTask"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        let flow = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(TICK_SECONDS));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                flow.tick().await;
            }
        });
        info!("[ai] 计划任务调度已启动, tick={}s", TICK_SECONDS);
        Ok(())
    }
}

impl FlowPlanTask {
    async fn tick(&self) {
        let tasks = match self.ai_plan_task_dao.find_all().await {
            Ok(tasks) => tasks,
            Err(error) => {
                warn!("[ai] 读取任务列表失败: {:#}", error);
                return;
            }
        };
        let now = now_millis();
        for shared in tasks {
            let task = shared.read().clone();
            if !task.is_due(now) {
                continue;
            }
            let flow = self.clone();
            tokio::spawn(async move {
                if let Err(error) = flow.run_task(task.id.clone()).await {
                    warn!("[ai] 任务执行失败 task={} error={:#}", task.id, error);
                }
            });
        }
    }

    pub async fn run_task(&self, task_id: String) -> Result<AiPlanRun> {
        if self.running.insert(task_id.clone(), ()).is_some() {
            bail!("task is already running");
        }
        let result = self.do_run(&task_id).await;
        self.running.remove(&task_id);
        result
    }

    async fn do_run(&self, task_id: &str) -> Result<AiPlanRun> {
        let Some(shared) = self.ai_plan_task_dao.find_by_id(task_id.to_string()).await? else {
            bail!("plan task not found: {}", task_id);
        };
        let task = shared.read().clone();
        let started_at = now_millis();
        self.ai_plan_task_service.mark_run(task.id.clone(), started_at).await?;

        let chat = self.ai_chat_service.start_task(AiChatType::Plan, task.name.clone()).await?;
        let mut run = AiPlanRun::default();
        run.id = uuid();
        run.task_id = task.id.clone();
        run.chat_id = chat.id.clone();
        run.started_at = started_at;
        let saved = self.ai_plan_run_service.insert(run).await?;
        let mut run = saved.read().clone();
        self.emit_changed(&task.id);
        info!("[ai] 任务开始 task={} chat={}", task.id, chat.id);

        let context = Arc::new(ReqChatContext {
            images: Vec::new(),
            thinking_level: None,
            origin: ChatOrigin::Plan,
            ai_chat_id: chat.id.clone(),
            confirm_mode: ConfirmMode::Silent,
            message: Self::build_message(&task),
            ai_provide_id: task.ai_provide_id.clone(),
        });
        let req_id = uuid();
        self.run_req.insert(run.id.clone(), req_id.clone());
        let result = self.agent_chat.chat_with_context(req_id, context).await;
        self.run_req.remove(&run.id);

        run.ended_at = now_millis();
        match result {
            Ok(()) => {
                run.status = AiPlanRunStatus::Success as i16;
                run.summary = self.resolve_summary(&chat.id).await;
            }
            Err(error) => {
                run.status = AiPlanRunStatus::Failed as i16;
                run.error = Some(format!("{:#}", error));
            }
        }
        let saved = self.ai_plan_run_service.update(run).await?;
        let run = saved.read().clone();
        self.emit_changed(&task.id);
        info!("[ai] 任务结束 task={} chat={} status={}", task.id, chat.id, run.status);
        Ok(run)
    }

    fn build_message(task: &AiPlanTask) -> String {
        if cfg!(any(target_os = "ios", target_os = "android")) {
            return format!("{}\n\n[Workspace]This task is running on mobile. The workspace directory may be invalid (especially in the iOS sandbox). Do not write files there and do not put task outputs in the workspace directory.", task.prompt);
        }
        let Some(dir) = task.workspace_dir.as_deref().map(str::trim).filter(|dir| !dir.is_empty()) else {
            return task.prompt.clone();
        };
        format!("{}\n\n[Workspace]This task's workspace directory is {}. Write all task outputs (reports/documents/spreadsheets) into that directory. Create the directory if it does not exist.", task.prompt, dir)
    }

    async fn resolve_summary(&self, chat_id: &str) -> Option<String> {
        let items = match self.ai_chat_item_service.select_by_chat_id(chat_id.to_string()).await {
            Ok(items) => items,
            Err(error) => {
                warn!("[ai] 读取会话内容失败 chat={} error={:#}", chat_id, error);
                return None;
            }
        };
        let text = items
            .iter()
            .rev()
            .find(|item| item.read().r#type == RespEventAgent::ON_TEXT_FINAL)
            .and_then(|item| item.read().context.get("delta").and_then(|value| value.as_str()).map(str::to_string))?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.chars().take(SUMMARY_MAX_CHARS).collect())
    }

    /// 取消执行中的运行：按 run_id 找到本次会话的 req_id 并中断，已结束的运行无事可做
    pub fn cancel_run(&self, run_id: &str) -> Result<()> {
        let Some(entry) = self.run_req.get(run_id) else {
            return Ok(());
        };
        self.agent_chat.cancel(entry.value())
    }

    fn emit_changed(&self, task_id: &str) {
        self.ws_manager.broadcast_all(&RespEventAiPlan::OnChanged(task_id.to_string()).to_resp());
    }
}
