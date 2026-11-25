use std::sync::Arc;
use tauri::Manager;
use anyhow::Context;
use parking_lot::Mutex;
use std::path::PathBuf;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::{AppContext, BaseApp};

#[termini_macros::base_app(sort = 90)]
#[derive(Clone, Service)]
pub struct AppConfig {
    #[inject(component)]
    handle: tauri::AppHandle,

    #[inject(func = Arc::new(Mutex::new(None)))]
    data_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl AppConfig {
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.lock().clone().expect("app config not initialized, call start() first")
    }
}

#[async_trait]
impl BaseApp for AppConfig {
    fn phase(&self) -> i16 {
        5
    }

    fn name(&self) -> &'static str {
        "AppConfig"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let data_dir = self.handle.path().app_data_dir().context("resolve app_data_dir")?;
        std::fs::create_dir_all(&data_dir).with_context(|| format!("create data dir {:?}", data_dir))?;
        let path_str: String = data_dir.to_string_lossy().to_string();
        *self.data_dir.lock() = Some(data_dir);
        info!("[config] App 数据目录[{}].", path_str);
        Ok(())
    }
}
