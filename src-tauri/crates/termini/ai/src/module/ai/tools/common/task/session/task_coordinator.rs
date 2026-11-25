use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use std::time::Duration;
use tokio::sync::Notify;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::util::time_util::now_millis;
use tokio_util::sync::CancellationToken;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::module::ai::tools::common::task::session::task_registry::TaskEntry;

pub const WAKE_GRACE_SECS: i64 = 30;
pub const BLOCK_MAX_SECS: i64 = 240;
const TICK: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    Finished,
    Detached,
    Cancelled,
}

struct Member {
    entry: TaskEntry,
    block_max_secs: i64,
    released: Arc<Notify>,
    tool_cancel: CancellationToken,
    outcome: Arc<Mutex<Option<TaskOutcome>>>,
}

pub struct TaskBatch {
    changed: Notify,
    loop_active: AtomicBool,
    members: Mutex<Vec<Member>>,
    session_cancel: CancellationToken,
    first_finished_ms: Mutex<Option<i64>>,
}

impl TaskBatch {
    fn new(session_cancel: CancellationToken) -> Self {
        Self { members: Mutex::new(Vec::new()), changed: Notify::new(), loop_active: AtomicBool::new(false), session_cancel, first_finished_ms: Mutex::new(None) }
    }

    fn resolve(member: &Member, outcome: TaskOutcome) {
        *member.outcome.lock() = Some(outcome);
        member.released.notify_one();
    }

    fn settle(&self) {
        let now = now_millis();
        let mut members = self.members.lock();
        if members.is_empty() {
            return;
        }
        let session_cancelled = self.session_cancel.is_cancelled();
        let mut first_finished = self.first_finished_ms.lock();
        for member in members.iter() {
            if member.entry.state().is_finished() && first_finished.is_none() {
                *first_finished = Some(now);
            }
        }
        let grace_over = first_finished.map(|at| now.saturating_sub(at) / 1000 >= WAKE_GRACE_SECS).unwrap_or(false);
        let block_over = members.iter().any(|member| now.saturating_sub(member.entry.started_ms) / 1000 >= member.block_max_secs);
        let detach_rest = grace_over || block_over;

        members.retain(|member| {
            let outcome = if session_cancelled || member.tool_cancel.is_cancelled() {
                Some(TaskOutcome::Cancelled)
            } else if member.entry.state().is_finished() {
                Some(TaskOutcome::Finished)
            } else if detach_rest {
                Some(TaskOutcome::Detached)
            } else {
                None
            };
            match outcome {
                Some(outcome) => {
                    if outcome == TaskOutcome::Detached {
                        member.entry.mark_detached();
                        info!(
                            "[ai] {}转后台 task_id={} elapsed={}s reason={}",
                            member.entry.kind.label(),
                            member.entry.task_id,
                            member.entry.elapsed_secs(),
                            if grace_over { "同批已有结果到手" } else { "阻塞到上限" }
                        );
                    }
                    Self::resolve(member, outcome);
                    false
                }
                None => true,
            }
        });
        if members.is_empty() {
            *first_finished = None;
        }
    }

    async fn wait_change(&self) {
        let (finished, cancels): (Vec<Arc<Notify>>, Vec<CancellationToken>) = {
            let members = self.members.lock();
            (members.iter().map(|member| member.entry.finished.clone()).collect(), members.iter().map(|member| member.tool_cancel.clone()).collect())
        };
        let any_finished = async {
            if finished.is_empty() {
                std::future::pending::<()>().await;
            }
            let mut futures: Vec<_> = finished.iter().map(|notify| Box::pin(notify.notified())).collect();
            for future in futures.iter_mut() {
                future.as_mut().enable();
            }
            if self.members.lock().iter().any(|member| member.entry.state().is_finished()) {
                return;
            }
            futures::future::select_all(futures).await;
        };
        let any_cancelled = async {
            if cancels.is_empty() {
                std::future::pending::<()>().await;
            }
            let futures: Vec<_> = cancels.iter().map(|token| Box::pin(token.cancelled())).collect();
            futures::future::select_all(futures).await;
        };
        tokio::select! {
            biased;
            _ = self.session_cancel.cancelled() => {}
            _ = self.changed.notified() => {}
            _ = any_finished => {}
            _ = any_cancelled => {}
            _ = tokio::time::sleep(TICK) => {}
        }
    }

    async fn run_loop(self: Arc<Self>) {
        loop {
            self.settle();
            {
                // 与 join 里的 push+swap 同在 members 锁下, 保证要么本循环看见新成员, 要么 join 看见循环已退出而重新拉起
                let members = self.members.lock();
                if members.is_empty() {
                    self.loop_active.store(false, Ordering::SeqCst);
                    return;
                }
            }
            self.wait_change().await;
        }
    }
}

fn create_batches() -> Arc<DashMap<String, Arc<TaskBatch>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct TaskCoordinator {
    #[inject(func = create_batches())]
    batches: Arc<DashMap<String, Arc<TaskBatch>>>,
}

#[async_trait]
impl BaseApp for TaskCoordinator {
    fn name(&self) -> &'static str {
        "TaskCoordinator"
    }
}

impl TaskCoordinator {
    pub async fn join(&self, parent_req_id: &str, session_cancel: CancellationToken, entry: TaskEntry, tool_cancel: CancellationToken, block_max_secs: i64) -> TaskOutcome {
        let batch = self.batches.entry(parent_req_id.to_string()).or_insert_with(|| Arc::new(TaskBatch::new(session_cancel.clone()))).clone();
        let outcome = Arc::new(Mutex::new(None));
        let released = Arc::new(Notify::new());
        {
            let mut members = batch.members.lock();
            members.push(Member { entry, tool_cancel, block_max_secs, outcome: outcome.clone(), released: released.clone() });
            if !batch.loop_active.swap(true, Ordering::SeqCst) {
                tokio::spawn(Arc::clone(&batch).run_loop());
            }
        }
        batch.changed.notify_one();
        loop {
            if let Some(result) = outcome.lock().clone() {
                return result;
            }
            released.notified().await;
        }
    }

    pub fn release(&self, parent_req_id: &str) {
        self.batches.remove(parent_req_id);
    }
}
