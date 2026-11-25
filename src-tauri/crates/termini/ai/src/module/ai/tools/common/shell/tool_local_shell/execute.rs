use regex::Regex;
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::Duration;
use rig::tool::ToolContext;
use crate::model::device::Device;
use crate::util::id_factory::uuid;
use std::sync::atomic::AtomicBool;
use summer::tracing::{info, warn};
use crate::module::ai::dto::AiToolId;
use crate::util::time_util::now_millis;
use tokio_util::sync::CancellationToken;
use crate::module::ai::dto::AiRunContext;
use crate::model::dto::shared_ptr::Shared;
use crate::module::ai::tools::base::WrapItem;
use crate::module::ai::dto::ShellSuperviseAction;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::shell::client_shell::ShellExit;
use crate::module::shell::client_shell::ClientShell;
use crate::model::req::req_chat_context::ReqChatContext;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::common::context::context_base::WindowSize;
use crate::module::ai::shared::chat_item_format::truncate_chars;
use crate::module::ai::tools::common::shell::local_shell_registry::ShellProcessEntry;
use crate::module::common::context::context_shell::{ConfigShell, ContextShell, ShellDescr};
use crate::module::ai::tools::common::shell::tool_local_shell::{ShellCommandSpec, ToolLocalShell};
use crate::module::ai::tools::common::shell::local_shell_wake::{sleep_or_wake, wrap_chunk_callback};
use crate::module::ai::flow::flow_shell_supervise::{SUPERVISE_FAIL_MAX, SUPERVISE_FAIL_RETRY_WAIT, SUPERVISE_FIRST_WAIT, SUPERVISE_TOTAL_MAX_SECS, SUPERVISE_WAIT_MAX_SECS, SUPERVISE_WAIT_MIN_SECS};

const BANNER_TITLE_CHARS: usize = 60;

pub(super) struct StepContext {
    pub(super) call_id: String,
    pub(super) shell_id: String,
    pub(super) cpu_percent: i16,
    pub(super) shell_descr: ShellDescr,
    pub(super) run: Arc<AiRunContext>,
    pub(super) chat_context: Arc<ReqChatContext>,
    pub(super) tool_cancel: CancellationToken,
    pub(super) session_cancel: CancellationToken,
}

impl StepContext {
    pub(super) fn chat_id(&self) -> String {
        self.chat_context.ai_chat_id.clone()
    }

    pub(super) fn shell_context(&self, spec: &ShellCommandSpec) -> ContextShell {
        let config = ConfigShell { cwd: spec.cwd.clone(), shell_descr: self.shell_descr.clone(), env: Vec::new(), cpu_percent: self.cpu_percent };
        ContextShell { size: WindowSize::default(), config: Shared::new(config), device: Shared::new(Device::new_local()) }
    }
}

struct StepEnd {
    body: String,
    stop: bool,
}

impl ToolLocalShell {
    pub(super) async fn run_batch(&self, context: &mut ToolContext, step: &StepContext, specs: &[ShellCommandSpec]) -> Result<String, AiToolError> {
        let total = specs.len();
        let mut round = 0usize;
        let mut items: Vec<WrapItem> = Vec::with_capacity(total + 1);
        let mut transcript: Vec<String> = Vec::with_capacity(total + 1);
        for (index, spec) in specs.iter().enumerate() {
            let banner = Self::step_banner(index, total, spec);
            if let Some(banner) = banner.as_deref() {
                self.push_banner_chunk(step, banner);
            }
            let end = self.run_step(step, spec, &mut round).await?;
            items.push(WrapItem::wrap_item(banner.clone(), end.body.clone()));
            transcript.push(Self::transcript_part(banner.as_deref(), &end.body));
            if !end.stop {
                continue;
            }
            if let Some(skipped) = Self::skipped_item(index, total, specs) {
                let stopped = format!("===== {}/{} stopped =====", index + 1, total);
                transcript.push(Self::transcript_part(Some(stopped.as_str()), &skipped));
                items.push(WrapItem::wrap_item(Some(stopped), skipped));
            }
            break;
        }
        self.tool_call_done_push.push_tool_call_done_event(&step.run, &step.call_id, transcript.join("\n"));
        context.insert_result(items);
        Ok(String::new())
    }

    async fn run_step(&self, step: &StepContext, spec: &ShellCommandSpec, round: &mut usize) -> Result<StepEnd, AiToolError> {
        let wake_regex = match spec.wake_pattern.as_deref() {
            Some(pattern) => Some(Regex::new(pattern).map_err(|error| AiToolError::Execution(format!("wake_pattern is not a valid regex: {}", error)))?),
            None => None,
        };
        let process_id = uuid();
        let started_ms = now_millis();
        let last_at_ms = Arc::new(Mutex::new(None));
        let resident = Arc::new(AtomicBool::new(false));
        let (on_chunk, chunk_buffer) = self.tool_call_chunk_push.build_shell_chunk_callback(&step.run, &step.call_id, self.tool_id().as_str());
        let wake_on_chunk = wrap_chunk_callback(on_chunk, chunk_buffer.clone(), last_at_ms.clone());
        let spawn_on_chunk = self.shell_daemon_push.wrap_chunk_callback(wake_on_chunk, chunk_buffer.clone(), step.chat_id(), process_id.clone(), resident.clone());
        let client = ClientShell::new();
        let shell_context = step.shell_context(spec);
        let process = client.spawn(&shell_context, spec.command.clone(), spawn_on_chunk).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let entry = ShellProcessEntry {
            process,
            started_ms,
            last_at_ms,
            resident,
            process_id,
            chunk_buffer,
            pty: None,
            cwd: spec.cwd.clone(),
            chat_id: step.chat_id(),
            call_id: step.call_id.clone(),
            command: spec.command.clone(),
            shell_id: step.shell_id.clone(),
        };
        self.local_shell_registry.insert(entry.clone());
        if let Some(regex) = wake_regex {
            let reason = format!("wake_pattern={} declared, mounted as a background wake source without supervision", regex.as_str());
            let body = self.finish_resident(&entry, step.chat_context.clone(), Some(regex), &reason);
            return Ok(StepEnd { body, stop: false });
        }
        self.supervise_step(step, &entry, spec.resident, round).await
    }

    async fn supervise_step(&self, step: &StepContext, entry: &ShellProcessEntry, resident: bool, round: &mut usize) -> Result<StepEnd, AiToolError> {
        let mut since_seq = 0u64;
        let mut supervise_fails = 0usize;
        let mut wait = SUPERVISE_FIRST_WAIT;
        let mut wake_patterns: Vec<String> = Vec::new();
        let mut process = entry.process.clone();
        loop {
            tokio::select! {
                biased;
                _ = step.tool_cancel.cancelled() => return Ok(StepEnd { body: self.finish_cancelled(entry), stop: true }),
                _ = step.session_cancel.cancelled() => return Ok(StepEnd { body: self.finish_cancelled(entry), stop: true }),
                exit = process.wait_exit() => {
                    let exit = exit.map_err(|error| AiToolError::Execution(error.to_string()))?;
                    return Ok(self.finish_step_exit(entry, exit));
                }
                hit = sleep_or_wake(entry, wait, &wake_patterns, since_seq as usize) => {
                    if let Some(pattern) = hit {
                        info!("[ai] 输出命中唤醒模式, 提前结束等待 process_id={} pattern={}", entry.process_id, pattern);
                    }
                }
            }
            if entry.elapsed_secs() >= SUPERVISE_TOTAL_MAX_SECS {
                let reason = format!("still running after {}s", entry.elapsed_secs());
                let body = self.finish_detached(entry, step.chat_context.clone(), resident, &reason);
                return Ok(StepEnd { body, stop: false });
            }
            *round += 1;
            let decided = tokio::select! {
                biased;
                _ = step.tool_cancel.cancelled() => return Ok(StepEnd { body: self.finish_cancelled(entry), stop: true }),
                _ = step.session_cancel.cancelled() => return Ok(StepEnd { body: self.finish_cancelled(entry), stop: true }),
                exit = process.wait_exit() => {
                    let exit = exit.map_err(|error| AiToolError::Execution(error.to_string()))?;
                    return Ok(self.finish_step_exit(entry, exit));
                }
                decided = self.flow_shell_supervise.decide(&step.run, entry, &step.shell_id, *round, since_seq) => decided,
            };
            let decision = match decided {
                Ok(decision) => {
                    supervise_fails = 0;
                    decision
                }
                Err(error) => {
                    supervise_fails += 1;
                    if supervise_fails >= SUPERVISE_FAIL_MAX {
                        let reason = format!("supervise model unavailable for {} rounds", supervise_fails);
                        warn!("[ai] 监督模型连续不可用, 命令转后台 req_id={} process_id={} error={}", step.run.req_id, entry.process_id, error);
                        let body = self.finish_detached(entry, step.chat_context.clone(), resident, &reason);
                        return Ok(StepEnd { body, stop: false });
                    }
                    warn!(
                        "[ai] 监督模型不可用，命令继续执行稍后重试监督 req_id={} process_id={} round={} fails={} error={}",
                        step.run.req_id, entry.process_id, round, supervise_fails, error
                    );
                    wait = SUPERVISE_FAIL_RETRY_WAIT;
                    continue;
                }
            };
            since_seq = entry.chunk_buffer.lock().len() as u64;
            info!(
                "[ai] 监督判定 req_id={} process_id={} round={} action={:?} wait_seconds={} reason={}",
                step.run.req_id, entry.process_id, round, decision.action, decision.wait_seconds, decision.reason
            );
            self.tool_supervise_push.push_tool_supervise_event(&step.run, &step.call_id, self.tool_id().as_str(), *round, &decision);
            match decision.action {
                ShellSuperviseAction::Wait => {
                    wait = Duration::from_secs(decision.wait_seconds.clamp(SUPERVISE_WAIT_MIN_SECS, SUPERVISE_WAIT_MAX_SECS));
                    wake_patterns = decision.wake_patterns.clone();
                }
                ShellSuperviseAction::Kill => {
                    let body = self.finish_killed(entry, &decision.reason).await;
                    return Ok(StepEnd { body, stop: true });
                }
                ShellSuperviseAction::Detach => {
                    let body = self.finish_detached(entry, step.chat_context.clone(), resident, &decision.reason);
                    return Ok(StepEnd { body, stop: false });
                }
            }
        }
    }

    fn finish_step_exit(&self, entry: &ShellProcessEntry, exit: ShellExit) -> StepEnd {
        let stop = entry.process.kill_requested() || !matches!(exit, ShellExit::Code(0));
        StepEnd { body: self.finish_exited(entry, exit), stop }
    }

    fn push_banner_chunk(&self, step: &StepContext, banner: &str) {
        let delta = format!("\n{}\n", banner);
        if let Err(error) = self.tool_call_chunk_push.push(&step.run, &step.call_id, AiToolId::ToolLocalShell.as_str(), delta) {
            warn!("[ai] 推送批次分隔失败 call_id={} error={:#}", step.call_id, error);
        }
    }

    fn step_banner(index: usize, total: usize, spec: &ShellCommandSpec) -> Option<String> {
        if total <= 1 {
            return None;
        }
        Some(format!("===== {}/{} {} =====", index + 1, total, Self::step_label(spec)))
    }

    fn step_label(spec: &ShellCommandSpec) -> String {
        let label = match spec.title.as_deref().map(str::trim).filter(|title| !title.is_empty()) {
            Some(title) => title.to_string(),
            None => Self::first_line(&spec.command),
        };
        truncate_chars(&label, BANNER_TITLE_CHARS)
    }

    fn first_line(command: &str) -> String {
        command.lines().find(|line| !line.trim().is_empty()).unwrap_or_default().trim().to_string()
    }

    fn transcript_part(banner: Option<&str>, body: &str) -> String {
        match banner {
            Some(banner) => format!("{}\n{}", banner, body),
            None => body.to_string(),
        }
    }

    fn skipped_item(index: usize, total: usize, specs: &[ShellCommandSpec]) -> Option<String> {
        let rest = specs.get(index + 1..)?;
        if rest.is_empty() {
            return None;
        }
        let lines: Vec<String> = rest.iter().map(|spec| truncate_chars(&Self::first_line(&spec.command), BANNER_TITLE_CHARS)).collect();
        Some(format!("command {}/{} did not succeed, so the batch stopped here and the remaining {} command(s) were not executed:\n{}", index + 1, total, lines.len(), lines.join("\n")))
    }
}
