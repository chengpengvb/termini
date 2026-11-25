use base64::Engine;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use rquickjs::{Context, Function, Runtime};
use crate::module::ai::tools::base::PathGuard;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;

const SHIMS: &str = include_str!("../../../../../../resource/script/js_shims.js");
const LIB_DOCX: &str = include_str!("../../../../../../resource/script/vendor/docx.umd.js");
const LIB_PPTX: &str = include_str!("../../../../../../resource/script/vendor/pptxgen.bundle.js");

const MEMORY_LIMIT: usize = 512 * 1024 * 1024;
const SETTLE_SPIN_MAX: usize = 100_000;

pub struct JsScript;

impl JsScript {
    pub fn run(code: &str, cwd: &Path, args: &[String], output: &UnboundedSender<Vec<u8>>, cancel: &Arc<AtomicBool>) -> Result<()> {
        let runtime = Runtime::new().map_err(|error| anyhow!("failed to create js runtime: {}", error))?;
        runtime.set_memory_limit(MEMORY_LIMIT);
        let interrupt = Arc::clone(cancel);
        runtime.set_interrupt_handler(Some(Box::new(move || interrupt.load(Ordering::Relaxed))));
        let context = Context::full(&runtime).map_err(|error| anyhow!("failed to create js context: {}", error))?;

        let print_tx = output.clone();
        let write_root = cwd.to_path_buf();
        let read_root = cwd.to_path_buf();
        let script_args = args.to_vec();

        context.with(|ctx| -> Result<()> {
            let globals = ctx.globals();
            let print = Function::new(ctx.clone(), move |line: String| {
                let _ = print_tx.send(format!("{}\n", line).into_bytes());
            })
            .map_err(|error| anyhow!("bind __print: {}", error))?;
            globals.set("__print", print).map_err(|error| anyhow!("set __print: {}", error))?;

            let write = Function::new(ctx.clone(), move |path: String, base64: String| write_file(&write_root, &path, &base64)).map_err(|error| anyhow!("bind __write_file: {}", error))?;
            globals.set("__write_file", write).map_err(|error| anyhow!("set __write_file: {}", error))?;

            let read = Function::new(ctx.clone(), move |path: String| read_file(&read_root, &path)).map_err(|error| anyhow!("bind __read_file: {}", error))?;
            globals.set("__read_file", read).map_err(|error| anyhow!("set __read_file: {}", error))?;

            eval_stage(&ctx, SHIMS, "js_shims.js")?;
            eval_stage(&ctx, LIB_DOCX, "docx.umd.js")?;
            eval_stage(&ctx, LIB_PPTX, "pptxgen.bundle.js")?;
            let args_json = serde_json::to_string(&script_args).map_err(|error| anyhow!("encode args: {}", error))?;
            eval_stage(&ctx, &format!("globalThis.args = {};", args_json), "args")?;
            eval_stage(&ctx, &wrap_user_code(code), "script")?;
            Ok(())
        })?;

        let mut spins = 0usize;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(anyhow!("script cancelled"));
            }
            while runtime.execute_pending_job().unwrap_or(false) {
                if cancel.load(Ordering::Relaxed) {
                    return Err(anyhow!("script cancelled"));
                }
            }
            let drained: i32 = context.with(|ctx| ctx.eval("__drainTimers()").unwrap_or(0));
            if drained == 0 {
                break;
            }
            spins += 1;
            if spins > SETTLE_SPIN_MAX {
                return Err(anyhow!("script did not settle after {} timer rounds", SETTLE_SPIN_MAX));
            }
        }

        let failure: Option<String> = context.with(|ctx| ctx.eval("globalThis.__error || null").ok());
        match failure {
            Some(message) => Err(anyhow!("{}", message)),
            None => Ok(()),
        }
    }
}

pub async fn execute(
    code: String,
    cwd: Option<String>,
    args: Vec<String>,
    on_chunk: ShellChunkCallback,
    tool_cancel: CancellationToken,
    session_cancel: CancellationToken,
    timeout: Duration,
) -> String {
    let directory = match resolve_cwd(cwd) {
        Ok(value) => value,
        Err(error) => return format!("exit_code=error runtime=js\n{}", error),
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let forward = tokio::spawn(async move {
        while let Some(data) = chunk_rx.recv().await {
            on_chunk(data).await;
        }
    });
    let run_cancel = Arc::clone(&cancel);
    let mut task = tokio::task::spawn_blocking(move || JsScript::run(&code, &directory, &args, &chunk_tx, &run_cancel));
    let outcome = tokio::select! {
        biased;
        _ = tool_cancel.cancelled() => None,
        _ = session_cancel.cancelled() => None,
        _ = tokio::time::sleep(timeout) => None,
        result = &mut task => Some(result),
    };
    let tail = match outcome {
        Some(Ok(Ok(()))) => "exit_code=0 runtime=js".to_string(),
        Some(Ok(Err(error))) => format!("exit_code=error runtime=js\n{}", error),
        Some(Err(error)) => format!("exit_code=error runtime=js\n{}", error),
        None => {
            cancel.store(true, Ordering::Relaxed);
            let _ = task.await;
            "exit_code=cancelled runtime=js".to_string()
        }
    };
    let _ = forward.await;
    tail
}

fn resolve_cwd(cwd: Option<String>) -> Result<PathBuf> {
    if let Some(value) = cwd.map(|item| item.trim().to_string()).filter(|item| !item.is_empty()) {
        return Ok(PathBuf::from(value));
    }
    #[cfg(target_os = "ios")]
    {
        let dir = termini_platform::util::workspace::workspace_dir()?;
        std::fs::create_dir_all(&dir).map_err(|error| anyhow!("create workspace dir: {}", error))?;
        Ok(dir)
    }
    #[cfg(not(target_os = "ios"))]
    {
        std::env::current_dir().map_err(|error| anyhow!("current dir: {}", error))
    }
}

fn wrap_user_code(code: &str) -> String {
    format!("(async function () {{\n{}\n}})().catch(function (error) {{ globalThis.__error = __describeError(error); }});", code)
}

fn eval_stage(ctx: &rquickjs::Ctx<'_>, source: &str, label: &str) -> Result<()> {
    ctx.eval::<(), _>(source.as_bytes()).map_err(|error| {
        let detail = ctx.catch().as_exception().map(|exception| exception.to_string()).unwrap_or_else(|| error.to_string());
        anyhow!("{} failed: {}", label, detail)
    })
}

fn resolve_target(root: &Path, path: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(path);
    let absolute = if candidate.is_absolute() { candidate } else { root.join(candidate) };
    PathGuard::ensure_writable(&absolute).map_err(|error| anyhow!("{}", error))?;
    Ok(absolute)
}

fn write_file(root: &Path, path: &str, base64: &str) -> String {
    match write_file_inner(root, path, base64) {
        Ok(resolved) => resolved,
        Err(error) => format!("\u{0}ERR:{}", error),
    }
}

fn write_file_inner(root: &Path, path: &str, base64: &str) -> Result<String> {
    let target = resolve_target(root, path)?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64.as_bytes()).map_err(|error| anyhow!("decode base64: {}", error))?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| anyhow!("create dir {}: {}", parent.display(), error))?;
    }
    std::fs::write(&target, &bytes).map_err(|error| anyhow!("write {}: {}", target.display(), error))?;
    Ok(target.to_string_lossy().into_owned())
}

fn read_file(root: &Path, path: &str) -> String {
    match read_file_inner(root, path) {
        Ok(value) => value,
        Err(error) => format!("\u{0}ERR:{}", error),
    }
}

fn read_file_inner(root: &Path, path: &str) -> Result<String> {
    let candidate = PathBuf::from(path);
    let absolute = if candidate.is_absolute() { candidate } else { root.join(candidate) };
    let bytes = std::fs::read(&absolute).map_err(|error| anyhow!("read {}: {}", absolute.display(), error))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}
