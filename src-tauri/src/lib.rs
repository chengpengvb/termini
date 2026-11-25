mod web;
mod base;
#[cfg(test)]
#[path = "../test/mod.rs"]
mod test;
mod module;
mod service;
use summer::App;
use anyhow::anyhow;
use std::sync::Arc;
use tauri::Manager;
use tauri::AppHandle;
use crate::base::fatal;
use futures::FutureExt;
use std::future::Future;
use termini_core::model;
use tokio::runtime::Builder;
use crate::base::crash_marker;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_collect::collect;
use std::sync::atomic::AtomicBool;
use termini_base::system::bootstrap;
use crate::base::base_app::AppContext;
use summer::plugin::ComponentRegistry;
use crate::web::web_server::WebServerPlugin;
use summer::plugin::MutableComponentRegistry;
pub use crate::module::telnet::core as telnet;
use termini_base::system::system_app::SystemApp;
use std::panic::{catch_unwind, AssertUnwindSafe};
pub use crate::module::ssh::client_ssh::ClientSsh;
use crate::web::interceptor::{auth_layer, cors_layer};
pub use crate::module::common::dto::ssh_auth_prompt::*;
pub use model::resp::event::resp_event_ssh::RespEventSsh;
pub use crate::module::common::dto::ssh_session::SshSession;
pub use crate::module::ssh::auth::auth_method::SshAuthMethod;
pub use crate::module::common::context::context_ssh::ContextSsh;
pub use crate::module::ssh::client_ssh_handler::ClientSshHandler;
pub use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
pub use crate::module::common::dto::ssh_session_key::SshSessionKey;
pub use crate::module::telnet::keepalive::keepalive_nop::KeepaliveNop;
pub use crate::module::ssh::client_session_factory::ClientSessionFactory;
use summer_web::{handler::auto_router, LayerConfigurator, WebConfigurator};
pub use model::resp::event::resp_event_ssh_auth::{OpenSshStep, RespSshOpenStepType};

const SUMMER_WORKER_THREADS: usize = 4;
const SUMMER_STACK_SIZE: usize = 32 * 1024 * 1024;
const APP_TOML: &str = include_str!("base/config/app.toml");

#[cfg(feature = "dhat")]
#[global_allocator]
static DHAT_ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(feature = "dhat")]
fn spawn_dhat_dump(profiler: dhat::Profiler) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(60));
        drop(profiler);
        std::process::exit(0);
    });
}

async fn handle_app_setup(ready: Arc<AtomicBool>, app: Arc<App>) -> summer::error::Result<String> {
    let ctx = AppContext {};
    let system = app.get_expect_component::<SystemApp>();
    bootstrap::wire_all(&app, &system);
    termini_base::system::system_app::set_global(system.clone());
    system.on_complete(&ctx, &ready).await;
    Ok("system ready".to_string())
}

fn resolve_app_toml() -> String {
    let data_dir = std::env::var("TERMINI_DATA_DIR").unwrap_or_default().replace('\\', "/");
    APP_TOML.replace("${TERMINI_DATA_DIR}", &data_dir)
}

fn spawn_summer(handle: AppHandle) {
    let fatal_handle = handle.clone();
    std::thread::Builder::new()
        .name("summer".to_string())
        .stack_size(SUMMER_STACK_SIZE)
        .spawn(move || {
            let rt =
                Builder::new_multi_thread().worker_threads(SUMMER_WORKER_THREADS).thread_name("summer-worker").thread_stack_size(SUMMER_STACK_SIZE).enable_all().build().expect("build summer runtime");
            if catch_unwind(AssertUnwindSafe(move || rt.block_on(app_setup(handle)))).is_err() {
                fatal::raise(&fatal_handle);
            }
        })
        .expect("spawn summer thread");
}

async fn app_setup(handle: AppHandle) {
    let ready = Arc::new(AtomicBool::new(false));
    let ready_for_sched = ready.clone();
    let ready_for_layer = ready.clone();
    let fatal_handle = handle.clone();
    let app_toml = resolve_app_toml();

    App::new()
        .add_component(handle)
        .use_config_str(&app_toml)
        .add_plugin(WebServerPlugin)
        .add_router(auto_router())
        .add_router_layer(move |router| {
            let router = auth_layer::install(router, ready_for_layer.clone());
            cors_layer::install(router)
        })
        .add_scheduler(move |app: Arc<App>| {
            let ready = ready_for_sched.clone();
            let fut = async move {
                match AssertUnwindSafe(handle_app_setup(ready, app)).catch_unwind().await {
                    Ok(result) => result,
                    Err(_) => {
                        fatal::raise(&fatal_handle);
                        Err(anyhow!("startup thread panicked, fatal error page has been pushed").into())
                    }
                }
            };
            Box::new(fut) as Box<dyn Future<Output = summer::error::Result<String>> + Send>
        })
        .run()
        .await;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    match rustls::crypto::ring::default_provider().install_default() {
        Ok(()) => {}
        Err(_) => {}
    }
    #[cfg(feature = "dhat")]
    spawn_dhat_dump(dhat::Profiler::new_heap());
    let builder = tauri::Builder::default();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
        if let Some(url) = argv.iter().find(|arg| arg.starts_with("termini://")) {
            crate::service::service_account_token::submit_deep_link_url(url.clone());
        }
        termini_platform::module::open_here::OpenHere::ingest_open_args(argv);
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }));
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
        termini_platform::module::open_here::OpenHere::ingest_open_args(argv);
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }));
    #[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), not(feature = "mas")))]
    let builder = builder.plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None));
    let builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_haptics::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_deep_link::init());
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let builder = builder.plugin(tauri_plugin_ios_webview_insets::init());
    builder
        .invoke_handler(tauri::generate_handler![fatal::fatal_error, fatal::recover_master_key, fatal::restart_app, crash_marker::take_last_crash])
        .setup(|tauri_app| {
            let data_dir = tauri_app.path().app_data_dir().expect("resolve app_data_dir");
            std::env::set_var("TERMINI_DATA_DIR", &data_dir);
            crash_marker::arm(&data_dir, tauri_app.package_info().version.to_string().as_str());
            fatal::install_panic_hook();
            spawn_summer(tauri_app.handle().clone());
            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            termini_platform::module::open_here::OpenHere::ingest_open_args(std::env::args().skip(1));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running alias application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = &event {
                if let Some(system) = termini_base::system::system_app::try_global() {
                    if !system.exited() {
                        api.prevent_exit();
                        let handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            system.on_exit(&AppContext {}).await;
                            handle.exit(0);
                        });
                    }
                }
            }
            if let tauri::RunEvent::Exit = &event {
                if let Ok(data_dir) = app_handle.path().app_data_dir() {
                    crash_marker::disarm(&data_dir);
                }
            }
            #[cfg(mobile)]
            if let tauri::RunEvent::WindowEvent { event: window_event, .. } = &event {
                match window_event {
                    tauri::WindowEvent::Suspended => {
                        #[cfg(target_os = "ios")]
                        termini_platform::util::ios_background_task::begin();
                    }
                    tauri::WindowEvent::Resumed => {
                        #[cfg(target_os = "ios")]
                        termini_platform::util::ios_background_task::end();
                        web::web_server::request_rebind();
                    }
                    _ => {}
                }
            }
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &event {
                termini_platform::module::open_here::OpenHere::ingest_open_args(urls.iter().map(|url| url.to_string()));
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        });
}
