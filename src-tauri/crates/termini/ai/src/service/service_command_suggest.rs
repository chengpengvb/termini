use anyhow::Result;
use std::sync::Arc;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use summer::tracing::{info, warn};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::service::service_ai_provide::AiProvideService;
use crate::model::resp::resp_model::{BizError, ResultCode};
use crate::module::ai::flow::flow_command_suggest::FlowCommandSuggest;
use crate::model::req::req_command_suggest::{ReqCommandSuggest, SUGGEST_REQ_COUNT_MAX, SUGGEST_REQ_COUNT_MIN};

const SUGGEST_MIN_INTERVAL: Duration = Duration::from_millis(0);

struct SuggestGate {
    run_id: u64,
    cancel: CancellationToken,
    last_finished: Option<Instant>,
}

fn create_suggest_gate() -> Arc<AsyncMutex<SuggestGate>> {
    Arc::new(AsyncMutex::new(SuggestGate { run_id: 0, last_finished: None, cancel: CancellationToken::new() }))
}

#[base_app(sort = 911)]
#[derive(Clone, Service)]
pub struct CommandSuggestService {
    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(component)]
    flow_command_suggest: FlowCommandSuggest,

    #[inject(func = create_suggest_gate())]
    gate: Arc<AsyncMutex<SuggestGate>>,
}

#[async_trait]
impl BaseApp for CommandSuggestService {
    fn name(&self) -> &'static str {
        "CommandSuggestService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl CommandSuggestService {
    pub async fn suggest(&self, req: ReqCommandSuggest, sink: UnboundedSender<Vec<String>>) -> Result<Vec<String>> {
        let started = Instant::now();
        let count = Self::resolve_count(req.suggest_count)?;
        let Some(provide_id) = self.resolve_provide_id().await? else {
            return Ok(vec![]);
        };
        let (run_id, cancel, wait_for) = self.begin_gate().await;
        if self.await_turn(&cancel, wait_for, started).await {
            return Ok(vec![]);
        }
        let Some(mut result) = self.suggest_once(&req, provide_id, count, &cancel, sink.clone()).await else {
            return self.cancel_run(run_id, started).await;
        };
        if result.as_ref().err().is_some_and(Self::is_model_not_found) && !cancel.is_cancelled() {
            result = match self.suggest_after_refresh(&req, count, &cancel, sink).await? {
                Some(next) => next,
                None => return self.cancel_run(run_id, started).await,
            };
        }
        self.finish_run(run_id, &req, result, started, &cancel).await
    }

    async fn resolve_provide_id(&self) -> Result<Option<String>> {
        let Some(config) = self.config_service.select_config().await else {
            return Ok(None);
        };
        if !config.suggest_enabled {
            return Ok(None);
        }
        if let Some(id) = config.suggest_ai_provide_id.filter(|id| !id.trim().is_empty()) {
            return Ok(Some(id));
        }
        self.config_service.refresh_system_config().await?;
        Ok(Some(self.require_suggest_id().await?))
    }

    async fn begin_gate(&self) -> (u64, CancellationToken, Duration) {
        let mut gate = self.gate.lock().await;
        gate.cancel.cancel();
        let cancel = CancellationToken::new();
        gate.cancel = cancel.clone();
        gate.run_id = gate.run_id.wrapping_add(1);
        let run_id = gate.run_id;
        let wait_for = gate.last_finished.map(|last| SUGGEST_MIN_INTERVAL.saturating_sub(last.elapsed())).unwrap_or(Duration::ZERO);
        (run_id, cancel, wait_for)
    }

    async fn await_turn(&self, cancel: &CancellationToken, wait_for: Duration, started: Instant) -> bool {
        if !wait_for.is_zero() {
            info!("[ai] 限流等待 {}ms", wait_for.as_millis());
            tokio::select! {
                _ = tokio::time::sleep(wait_for) => {}
                _ = cancel.cancelled() => {
                    info!("[ai] 等待期间被新请求取消 elapsed={}ms", started.elapsed().as_millis());
                    return true;
                }
            }
        }
        if cancel.is_cancelled() {
            info!("[ai] 调用前被新请求取消 elapsed={}ms", started.elapsed().as_millis());
            return true;
        }
        false
    }

    async fn suggest_once(&self, req: &ReqCommandSuggest, provide_id: String, count: usize, cancel: &CancellationToken, sink: UnboundedSender<Vec<String>>) -> Option<Result<Vec<String>>> {
        tokio::select! {
            result = self.flow_command_suggest.suggest(req, provide_id, count, cancel.clone(), sink) => Some(result),
            _ = cancel.cancelled() => None,
        }
    }

    async fn suggest_after_refresh(&self, req: &ReqCommandSuggest, count: usize, cancel: &CancellationToken, sink: UnboundedSender<Vec<String>>) -> Result<Option<Result<Vec<String>>>> {
        self.ai_provide_service.refresh_cloud().await?;
        self.config_service.refresh_system_config().await?;
        let provide_id = self.require_suggest_id().await?;
        Ok(self.suggest_once(req, provide_id, count, cancel, sink).await)
    }

    async fn cancel_run(&self, run_id: u64, started: Instant) -> Result<Vec<String>> {
        self.mark_finished(run_id).await;
        info!("[ai] 执行中被新请求取消 elapsed={}ms", started.elapsed().as_millis());
        Ok(vec![])
    }

    async fn finish_run(&self, run_id: u64, req: &ReqCommandSuggest, result: Result<Vec<String>>, started: Instant, cancel: &CancellationToken) -> Result<Vec<String>> {
        self.mark_finished(run_id).await;
        match result {
            Ok(list) => {
                let history_count = req.history.iter().filter(|line| !line.trim().is_empty()).count();
                info!("[ai] 连接类型={} 历史条数={} 候选数={} 耗时={}ms", req.terminal_kind, history_count, list.len(), started.elapsed().as_millis());
                Ok(list)
            }
            Err(_) if cancel.is_cancelled() => {
                info!("[ai] 执行中被新请求取消 耗时={}ms", started.elapsed().as_millis());
                Ok(vec![])
            }
            Err(err) => {
                warn!("[ai] 连接类型={} 失败 耗时={}ms err={}", req.terminal_kind, started.elapsed().as_millis(), err);
                Err(err)
            }
        }
    }

    async fn require_suggest_id(&self) -> Result<String> {
        let Some(updated) = self.config_service.select_config().await else {
            return Err(ResultCode::DefaultConfigNotFound.into());
        };
        let Some(id) = updated.suggest_ai_provide_id.filter(|id| !id.trim().is_empty()) else {
            return Err(ResultCode::SuggestProvideEmpty.into());
        };
        Ok(id)
    }

    fn is_model_not_found(err: &anyhow::Error) -> bool {
        err.downcast_ref::<BizError>().is_some_and(|biz| biz.code == ResultCode::AiModelNotFound.as_i32())
    }

    async fn mark_finished(&self, run_id: u64) {
        let mut gate = self.gate.lock().await;
        if gate.run_id == run_id {
            gate.last_finished = Some(Instant::now());
        }
    }

    fn resolve_count(suggest_count: i16) -> Result<usize> {
        if !(SUGGEST_REQ_COUNT_MIN..=SUGGEST_REQ_COUNT_MAX).contains(&suggest_count) {
            return Err(ResultCode::SuggestCountRange.into());
        }
        Ok(suggest_count as usize)
    }
}
