use url::Url;
use std::sync::Arc;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::util::crypto_util;
use sysinfo::{Disks, System};
use termini_macros::base_app;
use crate::util::file_open_util;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use crate::util::clipboard_file_util;
#[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), not(feature = "mas")))]
use crate::module::open_here::OpenHere;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use crate::model::config::SystemLanguage;
use crate::web::ws::ws_manager::WsManager;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::model::dto::system_info::SystemInfo;
use crate::model::resp::resp_model::ResultCode;
use crate::base::base_app::{AppContext, BaseApp};
use tauri_plugin_clipboard_manager::ClipboardExt;
use crate::model::req::req_system_util::ReqHaptic;
use crate::service::service_config::ConfigService;
use crate::service::service_notify_tray::NotifyTrayService;
use crate::model::resp::resp_platform_info::RespPlatformInfo;
use crate::model::resp::event::resp_event_window::RespEventWindow;
use crate::module::shell::platform::shell_discover::native_shell_platform;
use crate::model::config::{CLOSE_ACTION_ASK, CLOSE_ACTION_EXIT, CLOSE_ACTION_TRAY};
use tauri_plugin_haptics::{HapticsExt, ImpactFeedbackStyle, NotificationFeedbackType};

#[base_app(sort = 1000)]
#[derive(Clone, Service)]
pub struct SystemUtilServiceImpl {
    #[inject(component)]
    handle: AppHandle,

    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    notify_tray_service: NotifyTrayService,
}

#[async_trait]
impl BaseApp for SystemUtilServiceImpl {
    fn name(&self) -> &'static str {
        "SystemUtilServiceImpl"
    }

    fn phase(&self) -> i16 {
        301
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        self.watch_window_maximized();
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            #[cfg(not(feature = "mas"))]
            self.cleanup_dev_autostart();
            self.watch_window_close_requested();
            self.setup_tray().await?;
            #[cfg(not(feature = "mas"))]
            self.sync_open_here_menu().await?;
        }
        Ok(())
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn show_main_window(handle: &AppHandle) {
    let Some(window) = handle.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub(crate) fn show_main_window_with_attention(handle: &AppHandle) {
    show_main_window(handle);
    let Some(window) = handle.get_webview_window("main") else {
        return;
    };
    let _ = window.request_user_attention(Some(tauri::UserAttentionType::Informational));
}

impl SystemUtilServiceImpl {
    pub async fn notify(&self, title: &str, body: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Err(ResultCode::NameEmpty.into());
        }
        self.notify_tray_service.notify(title.trim(), body.trim())
    }

    pub async fn read_clipboard_local_paths(&self) -> Result<Vec<String>> {
        Ok(clipboard_file_util::read_clipboard_local_paths())
    }

    pub async fn write_clipboard_text(&self, text: &str) -> Result<()> {
        self.handle.clipboard().write_text(text).map_err(|_| anyhow::Error::from(ResultCode::ClipboardWriteFailed))
    }

    pub async fn read_clipboard_text(&self) -> Result<String> {
        self.handle.clipboard().read_text().map_err(|_| anyhow::Error::from(ResultCode::ClipboardReadFailed))
    }

    pub async fn open_url(&self, url: &str) -> Result<()> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(ResultCode::UrlEmpty.into());
        }
        let parsed = Url::parse(trimmed).map_err(|_| anyhow::Error::from(ResultCode::InvalidUrl))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(ResultCode::UnsupportedUrlScheme.into());
        }
        self.handle.opener().open_url(parsed.as_str(), None::<&str>).map_err(|_| anyhow::Error::from(ResultCode::OpenUrlFailed))
    }

    pub async fn open_local_path(&self, path: &str) -> Result<()> {
        let trimmed = Self::existing_local_path(path)?;
        file_open_util::open_local_file(trimmed, false).map_err(|_| anyhow::Error::from(ResultCode::OpenLocalPathFailed))
    }

    pub async fn reveal_local_path(&self, path: &str) -> Result<()> {
        let trimmed = Self::existing_local_path(path)?;
        file_open_util::reveal_local_file(trimmed).map_err(|_| anyhow::Error::from(ResultCode::OpenLocalPathFailed))
    }

    fn existing_local_path(path: &str) -> Result<&str> {
        let trimmed = path.trim();
        if trimmed.is_empty() || !std::path::Path::new(trimmed).exists() {
            return Err(ResultCode::LocalPathNotFound.into());
        }
        Ok(trimmed)
    }

    fn resolve_picked_path(raw: &str) -> Result<std::path::PathBuf> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("copy source path is empty"));
        }
        if !trimmed.starts_with("file://") {
            return Ok(std::path::PathBuf::from(trimmed));
        }
        let url = Url::parse(trimmed).map_err(|error| anyhow!("invalid source file url: {}: {}", trimmed, error))?;
        url.to_file_path().map_err(|_| anyhow!("source file url is not a local path: {}", trimmed))
    }

    pub async fn copy_local_into(&self, dest_dir: &str, sources: &[String]) -> Result<()> {
        summer::tracing::info!("[file] dest={} sources={}", dest_dir, sources.len());
        #[cfg(target_os = "ios")]
        crate::util::ios_local_dir::ensure_within_documents(dest_dir)?;
        let dest = std::path::Path::new(dest_dir);
        let dest_meta = tokio::fs::metadata(dest).await.map_err(|_| anyhow::Error::from(ResultCode::LocalPathNotFound))?;
        if !dest_meta.is_dir() {
            return Err(anyhow!("copy destination is not a directory: {}", dest_dir));
        }
        if sources.is_empty() {
            return Err(anyhow!("copy sources is empty"));
        }
        for source in sources {
            let source_path = Self::resolve_picked_path(source)?;
            let source_path = source_path.as_path();
            let source_meta = tokio::fs::metadata(source_path).await.map_err(|_| anyhow!("copy source not found: {}", source))?;
            if source_meta.is_dir() {
                return Err(anyhow!("directory import is not supported: {}", source));
            }
            let name = source_path.file_name().and_then(|value| value.to_str()).ok_or_else(|| anyhow!("invalid source file name: {}", source))?;
            let target = dest.join(name);
            if target.exists() {
                return Err(anyhow!("target already exists: {}", target.display()));
            }
            tokio::fs::copy(source_path, &target).await.map_err(|error| anyhow!("copy {} -> {}: {}", source, target.display(), error))?;
        }
        Ok(())
    }

    pub async fn haptic(&self, req: &ReqHaptic) -> Result<()> {
        let haptics = self.handle.haptics();
        match req.kind.as_str() {
            "selection" => haptics.selection_feedback().map_err(|_| anyhow::Error::from(ResultCode::HapticUnsupported)),
            "impact" => {
                let style = match req.style.as_deref() {
                    Some("light") => ImpactFeedbackStyle::Light,
                    Some("medium") => ImpactFeedbackStyle::Medium,
                    Some("heavy") => ImpactFeedbackStyle::Heavy,
                    Some("soft") => ImpactFeedbackStyle::Soft,
                    Some("rigid") => ImpactFeedbackStyle::Rigid,
                    Some(_) => return Err(ResultCode::HapticUnsupported.into()),
                    None => return Err(ResultCode::HapticUnsupported.into()),
                };
                haptics.impact_feedback(style).map_err(|_| anyhow::Error::from(ResultCode::HapticUnsupported))
            }
            "notification" => {
                let feedback_type = match req.notification_type.as_deref() {
                    Some("success") => NotificationFeedbackType::Success,
                    Some("warning") => NotificationFeedbackType::Warning,
                    Some("error") => NotificationFeedbackType::Error,
                    Some(_) => return Err(ResultCode::HapticUnsupported.into()),
                    None => return Err(ResultCode::HapticUnsupported.into()),
                };
                haptics.notification_feedback(feedback_type).map_err(|_| anyhow::Error::from(ResultCode::HapticUnsupported))
            }
            "vibrate" => {
                let Some(duration) = req.duration else {
                    return Err(ResultCode::HapticUnsupported.into());
                };
                haptics.vibrate(duration).map_err(|_| anyhow::Error::from(ResultCode::HapticUnsupported))
            }
            _ => Err(ResultCode::HapticUnsupported.into()),
        }
    }

    #[cfg(desktop)]
    pub async fn window_minimize(&self) -> Result<()> {
        self.main_window()?.minimize().map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))
    }

    #[cfg(not(desktop))]
    pub async fn window_minimize(&self) -> Result<()> {
        Err(ResultCode::WindowOpUnsupported.into())
    }

    #[cfg(desktop)]
    pub async fn window_request_attention(&self, active: bool) -> Result<()> {
        let attention = if active { Some(tauri::UserAttentionType::Informational) } else { None };
        self.main_window()?.request_user_attention(attention).map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))
    }

    #[cfg(not(desktop))]
    pub async fn window_request_attention(&self, _active: bool) -> Result<()> {
        Err(ResultCode::WindowOpUnsupported.into())
    }

    #[cfg(desktop)]
    pub async fn window_toggle_maximize(&self) -> Result<()> {
        let window = self.main_window()?;
        let is_maximized = window.is_maximized().map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))?;
        if is_maximized {
            window.unmaximize().map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))
        } else {
            window.maximize().map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))
        }
    }

    #[cfg(not(desktop))]
    pub async fn window_toggle_maximize(&self) -> Result<()> {
        Err(ResultCode::WindowOpUnsupported.into())
    }

    pub async fn window_close(&self, action: Option<i16>) -> Result<()> {
        match action {
            Some(action) => self.run_close_action(action).await,
            None => self.apply_close_action().await,
        }
    }

    async fn apply_close_action(&self) -> Result<()> {
        let action = match self.config_service.select_config().await {
            Some(config) => config.close_action,
            None => CLOSE_ACTION_EXIT,
        };
        if action == CLOSE_ACTION_ASK {
            self.ws_manager.broadcast_all(&RespEventWindow::OnCloseAsk.to_resp());
            return Ok(());
        }
        self.run_close_action(action).await
    }

    async fn run_close_action(&self, action: i16) -> Result<()> {
        if action == CLOSE_ACTION_TRAY {
            return self.main_window()?.hide().map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed));
        }
        if let Some(system) = termini_base::system::system_app::try_global() {
            system.on_exit(&AppContext {}).await;
        }
        self.handle.exit(0);
        Ok(())
    }

    pub async fn window_is_maximized(&self) -> Result<bool> {
        self.main_window()?.is_maximized().map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))
    }

    #[cfg(desktop)]
    pub async fn window_set_fullscreen(&self, fullscreen: bool) -> Result<()> {
        self.main_window()?.set_simple_fullscreen(fullscreen).map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))
    }

    #[cfg(not(desktop))]
    pub async fn window_set_fullscreen(&self, _fullscreen: bool) -> Result<()> {
        Err(ResultCode::WindowOpUnsupported.into())
    }

    pub async fn window_is_fullscreen(&self) -> Result<bool> {
        self.main_window()?.is_fullscreen().map_err(|_| anyhow::Error::from(ResultCode::WindowOpFailed))
    }

    fn main_window(&self) -> Result<tauri::WebviewWindow> {
        self.handle.get_webview_window("main").ok_or_else(|| anyhow::Error::from(ResultCode::MainWindowNotFound))
    }

    fn watch_window_maximized(&self) {
        let Some(window) = self.handle.get_webview_window("main") else {
            return;
        };
        let watched_window = window.clone();
        let ws_manager = self.ws_manager.clone();
        let last_maximized = Arc::new(AtomicBool::new(watched_window.is_maximized().unwrap_or(false)));
        window.on_window_event(move |event| {
            if !matches!(event, tauri::WindowEvent::Resized(_)) {
                return;
            }
            let Ok(is_maximized) = watched_window.is_maximized() else {
                return;
            };
            if last_maximized.swap(is_maximized, Ordering::SeqCst) == is_maximized {
                return;
            }
            ws_manager.broadcast_all(&RespEventWindow::OnMaximizedChanged(is_maximized).to_resp());
        });
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn watch_window_close_requested(&self) {
        let Some(window) = self.handle.get_webview_window("main") else {
            return;
        };
        let service = self.clone();
        window.on_window_event(move |event| {
            let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };
            api.prevent_close();
            let service = service.clone();
            tauri::async_runtime::spawn(async move {
                let _ = service.apply_close_action().await;
            });
        });
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    async fn setup_tray(&self) -> Result<()> {
        use tauri::tray::TrayIconBuilder;
        use tauri::menu::{MenuBuilder, MenuItemBuilder};
        use crate::model::resp::event::resp_event_window::{TRAY_ACTION_HOME, TRAY_ACTION_NEW_CHAT};

        let labels = match self.config_service.select_system_language().await {
            SystemLanguage::EnUs => ["Show Termini", "Open Home", "New Chat", "Quit"],
            SystemLanguage::ZhCn => ["显示主界面", "打开首页", "新建聊天", "退出"],
            SystemLanguage::JaJp => ["Terminiを表示", "ホームを開く", "新しいチャット", "終了"],
            SystemLanguage::KoKr => ["Termini 표시", "홈 열기", "새 채팅", "종료"],
            SystemLanguage::DeDe => ["Termini anzeigen", "Startseite öffnen", "Neuer Chat", "Beenden"],
            SystemLanguage::FrFr => ["Afficher Termini", "Ouvrir l'accueil", "Nouveau chat", "Quitter"],
            SystemLanguage::EsEs => ["Mostrar Termini", "Abrir inicio", "Nuevo chat", "Salir"],
            SystemLanguage::ItIt => ["Mostra Termini", "Apri home", "Nuova chat", "Esci"],
            SystemLanguage::PtPt => ["Mostrar Termini", "Abrir início", "Novo chat", "Sair"],
            SystemLanguage::NlNl => ["Termini tonen", "Startpagina openen", "Nieuwe chat", "Afsluiten"],
            SystemLanguage::ZhTw => ["顯示主界面", "打開首頁", "新建聊天", "結束"],
            SystemLanguage::HiIn => ["Termini दिखाएँ", "होम खोलें", "नई चैट", "बाहर निकलें"],
        };
        let show = MenuItemBuilder::with_id("tray_show", labels[0]).build(&self.handle)?;
        let home = MenuItemBuilder::with_id("tray_home", labels[1]).build(&self.handle)?;
        let new_chat = MenuItemBuilder::with_id("tray_new_chat", labels[2]).build(&self.handle)?;
        let quit = MenuItemBuilder::with_id("tray_quit", labels[3]).build(&self.handle)?;
        let menu = MenuBuilder::new(&self.handle).item(&show).item(&home).item(&new_chat).separator().item(&quit).build()?;
        let icon = self.handle.default_window_icon().cloned().ok_or_else(|| anyhow!("window icon missing, cannot create tray"))?;
        let ws_manager = self.ws_manager.clone();
        let builder = TrayIconBuilder::with_id("main").icon(icon).tooltip("Termini").menu(&menu);
        #[cfg(not(target_os = "macos"))]
        let builder = builder.show_menu_on_left_click(false);
        builder
            .on_menu_event(move |app, event| match event.id().as_ref() {
                "tray_show" => show_main_window(app),
                "tray_home" => {
                    show_main_window(app);
                    ws_manager.broadcast_all(&RespEventWindow::OnTrayAction(TRAY_ACTION_HOME).to_resp());
                }
                "tray_new_chat" => {
                    show_main_window(app);
                    ws_manager.broadcast_all(&RespEventWindow::OnTrayAction(TRAY_ACTION_NEW_CHAT).to_resp());
                }
                "tray_quit" => {
                    let handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(system) = termini_base::system::system_app::try_global() {
                            system.on_exit(&AppContext {}).await;
                        }
                        handle.exit(0);
                    });
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                    show_main_window(tray.app_handle());
                }
            })
            .build(&self.handle)
            .map_err(|error| anyhow!("failed to create tray: {}", error))?;
        Ok(())
    }

    pub async fn master_key_phrase(&self) -> Result<String> {
        Ok(crypto_util::export_phrase())
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    pub async fn save_open_here(&self, enabled: bool) -> Result<()> {
        self.config_service.save_open_here(enabled).await?;
        #[cfg(not(feature = "mas"))]
        self.sync_open_here_menu().await?;
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub async fn save_open_here(&self, _enabled: bool) -> Result<()> {
        Err(anyhow!("open-here is not supported on the current platform"))
    }

    #[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), not(feature = "mas")))]
    async fn sync_open_here_menu(&self) -> Result<()> {
        let Some(config) = self.config_service.select_config().await else {
            return Err(anyhow!("default config is not found"));
        };
        OpenHere::sync_menu(config.open_here_enabled, SystemLanguage::from_i16(config.system_language))
    }

    #[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), not(feature = "mas")))]
    pub async fn auto_start(&self) -> Result<bool> {
        use tauri_plugin_autostart::ManagerExt;
        self.handle.autolaunch().is_enabled().map_err(|error| anyhow!("failed to read auto-start state: {}", error))
    }

    #[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), not(feature = "mas")))]
    pub async fn save_auto_start(&self, enabled: bool) -> Result<()> {
        use tauri_plugin_autostart::ManagerExt;
        let manager = self.handle.autolaunch();
        if enabled {
            if tauri::is_dev() {
                return Err(anyhow!("auto-start is not supported in dev mode: it would register the debug executable with the system, which cannot reach the frontend after boot"));
            }
            manager.enable().map_err(|error| anyhow!("failed to enable auto-start: {}", error))
        } else {
            manager.disable().map_err(|error| anyhow!("failed to disable auto-start: {}", error))
        }
    }

    #[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), feature = "mas"))]
    pub async fn auto_start(&self) -> Result<bool> {
        Ok(false)
    }

    #[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), feature = "mas"))]
    pub async fn save_auto_start(&self, _enabled: bool) -> Result<()> {
        Ok(())
    }

    #[cfg(all(any(target_os = "windows", target_os = "macos", target_os = "linux"), not(feature = "mas")))]
    fn cleanup_dev_autostart(&self) {
        use summer::tracing::warn;
        use tauri_plugin_autostart::ManagerExt;
        if !tauri::is_dev() {
            return;
        }
        let manager = self.handle.autolaunch();
        if !matches!(manager.is_enabled(), Ok(true)) {
            return;
        }
        if !Self::autostart_targets_current_exe() {
            return;
        }
        match manager.disable() {
            Ok(()) => warn!("[system] 检测到开机自启项指向 debug 可执行文件, 已自动关闭"),
            Err(error) => warn!("[system] 关闭 debug 开机自启项失败: {}", error),
        }
    }

    #[cfg(all(target_os = "windows", not(feature = "mas")))]
    fn autostart_targets_current_exe() -> bool {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
        let Ok(current) = std::env::current_exe() else {
            return false;
        };
        let Ok(run) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_READ) else {
            return false;
        };
        let Ok(command) = run.get_value::<String, _>("Termini") else {
            return false;
        };
        let registered = command.trim().trim_matches('"').split('"').next().unwrap_or("").trim();
        let current = current.to_string_lossy();
        registered.eq_ignore_ascii_case(current.as_ref())
    }

    #[cfg(all(any(target_os = "macos", target_os = "linux"), not(feature = "mas")))]
    fn autostart_targets_current_exe() -> bool {
        // 这两个平台的自启项由 launch agent / desktop 文件承载, dev 环境下一律视为当前 exe。
        true
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub async fn auto_start(&self) -> Result<bool> {
        Ok(false)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub async fn save_auto_start(&self, _enabled: bool) -> Result<()> {
        Err(anyhow!("auto-start is not supported on the current platform"))
    }

    pub async fn platform_info(&self) -> Result<RespPlatformInfo> {
        let os = std::env::consts::OS.to_string();
        let has_web_login = !cfg!(feature = "mas");
        let mobile = matches!(os.as_str(), "ios" | "android");
        let has_local_shell = !native_shell_platform().discover().is_empty();
        let has_store = matches!(os.as_str(), "windows" | "macos" | "ios" | "android");
        Ok(RespPlatformInfo { os, mobile, has_local_shell, has_serial: !mobile, has_store, has_web_login })
    }

    pub async fn system_info(&self) -> Result<SystemInfo> {
        let platform_info = self.platform_info().await?;
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();
        Ok(SystemInfo {
            platform_info,
            disk_info: Self::resolve_disk_info(),
            cpu_info: Self::resolve_cpu_info(&sys),
            terminal: self.resolve_terminal().await,
            memory_info: Self::resolve_memory_info(&sys),
        })
    }

    fn resolve_cpu_info(sys: &System) -> String {
        let arch = System::cpu_arch();
        let brand = match sys.cpus().first() {
            Some(cpu) => {
                let brand = cpu.brand();
                if brand.is_empty() {
                    None
                } else {
                    Some(brand.to_string())
                }
            }
            None => None,
        };
        match (brand, System::physical_core_count()) {
            (Some(brand), Some(cores)) => format!("{} {} ({} cores)", brand, arch, cores),
            (Some(brand), None) => format!("{} {}", brand, arch),
            (None, Some(cores)) => format!("{} ({} cores)", arch, cores),
            (None, None) => arch,
        }
    }

    fn resolve_memory_info(sys: &System) -> String {
        let total_mb = sys.total_memory() / 1024 / 1024;
        let used_mb = sys.used_memory() / 1024 / 1024;
        format!("{} MB / {} MB", used_mb, total_mb)
    }

    fn resolve_disk_info() -> String {
        let disks = Disks::new_with_refreshed_list();
        let mut lines = Vec::new();
        for disk in disks.list() {
            let total_gb = disk.total_space() / 1024 / 1024 / 1024;
            let available_gb = disk.available_space() / 1024 / 1024 / 1024;
            lines.push(format!("{}: {} GB free / {} GB", disk.mount_point().display(), available_gb, total_gb));
        }
        lines.join("\n")
    }

    async fn resolve_terminal(&self) -> Option<String> {
        let Some(config) = self.config_service.select_config().await else {
            return None;
        };
        let Some(local_shell_id) = config.local_shell_id else {
            return None;
        };
        if local_shell_id.is_empty() {
            return None;
        }
        let Some(shell_descr) = native_shell_platform().find_by_shell_id(&local_shell_id) else {
            return Some(local_shell_id);
        };
        if shell_descr.description.is_empty() {
            return Some(local_shell_id);
        }
        Some(shell_descr.description)
    }
}
