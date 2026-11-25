use std::sync::Arc;
use anyhow::Context;
use std::path::PathBuf;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::config::app_config::AppConfig;
use crate::base::base_app::{AppContext, BaseApp};
use sea_orm::{ConnectOptions, Database as SeaDatabase, DatabaseConnection};

#[base_app(sort = 100)]
#[derive(Clone, Service)]
pub struct DataConnFactory {
    #[inject(component)]
    config: AppConfig,

    #[inject(func = Arc::new(parking_lot::Mutex::new(None)))]
    conn: Arc<parking_lot::Mutex<Option<DatabaseConnection>>>,
}

impl DataConnFactory {
    pub fn conn(&self) -> DatabaseConnection {
        self.conn.lock().as_ref().cloned().expect("database not initialized, call start() first")
    }

    fn resolve_db_url_from_dir(data_dir: PathBuf) -> anyhow::Result<String> {
        let path = data_dir.join("app.db");
        let path_str = path.to_string_lossy().replace('\\', "/");
        Ok(format!("sqlite:///{path_str}?mode=rwc"))
    }
}

#[async_trait]
impl BaseApp for DataConnFactory {
    fn phase(&self) -> i16 {
        10
    }

    fn name(&self) -> &'static str {
        "Database"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let data_dir = self.config.data_dir();
        let url = Self::resolve_db_url_from_dir(data_dir)?;
        let mut opt = ConnectOptions::new(url);
        opt.max_connections(10).min_connections(1).connect_timeout(std::time::Duration::from_secs(5)).sqlx_logging(false);

        let conn = SeaDatabase::connect(opt).await.context("connect database")?;
        *self.conn.lock() = Some(conn);
        Ok(())
    }
}
