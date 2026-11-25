use tauri::webview::Color;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::{AppContext, BaseApp};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

#[base_app(sort = 1100)]
#[derive(Clone, Service)]
pub struct WindowService {
    #[inject(component)]
    handle: AppHandle,
}

#[async_trait]
impl BaseApp for WindowService {
    fn name(&self) -> &'static str {
        "WindowService"
    }

    fn is_core(&self) -> bool {
        true
    }

    fn phase(&self) -> i16 {
        300
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        self.create_main_window()
    }
}

impl WindowService {
    fn create_main_window(&self) -> Result<()> {
        let handle = self.handle.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.handle
            .run_on_main_thread(move || {
                let result = WindowService::build_main(&handle);
                let _ = tx.send(result);
            })
            .map_err(|e| anyhow!("{e}"))?;
        match rx.recv() {
            Ok(result) => result,
            Err(e) => Err(anyhow!("{e}")),
        }
    }

    fn build_main(handle: &AppHandle) -> Result<()> {
        let builder = WebviewWindowBuilder::new(handle, "main", WebviewUrl::App("index.html".into())).title("Termini").accept_first_mouse(true).background_color(Color(0x0d, 0x11, 0x17, 0xff));
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        let builder = builder.inner_size(1280.0, 800.0).min_inner_size(1024.0, 720.0).decorations(false);
        builder.build().map(|_| ()).map_err(|e| anyhow!("{e}"))
    }
}
