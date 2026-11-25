use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use anyhow::{anyhow, Context, Result};
use crate::base::base_app::{AppContext, BaseApp};
use crate::dao::conn_factory::data_conn_factory::DataConnFactory;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

const MIGRATIONS: &[(i32, &str)] = &[
    (1, "ALTER TABLE config ADD COLUMN job_heartbeat_at bigint NOT NULL DEFAULT 0"),
    (2, "ALTER TABLE ai_provide ADD COLUMN vision boolean NOT NULL DEFAULT 0"),
    (3, "ALTER TABLE config ADD COLUMN vision_use_cloud boolean NOT NULL DEFAULT 1"),
    (4, "ALTER TABLE config ADD COLUMN vision_local_ai_provide_id varchar"),
    (5, "ALTER TABLE config ADD COLUMN image_gen_use_cloud boolean NOT NULL DEFAULT 1"),
    (6, "ALTER TABLE config ADD COLUMN image_gen_local_ai_provide_id varchar"),
    (7, "ALTER TABLE config ADD COLUMN embedding_use_cloud boolean NOT NULL DEFAULT 1"),
    (8, "ALTER TABLE config ADD COLUMN embedding_local_ai_provide_id varchar"),
    (9, "ALTER TABLE config ADD COLUMN embedding_local_dim integer"),
    (10, "ALTER TABLE config ADD COLUMN assist_use_cloud boolean NOT NULL DEFAULT 1"),
    (11, "ALTER TABLE config ADD COLUMN assist_local_ai_provide_id varchar"),
    (12, "ALTER TABLE config ADD COLUMN web_fetch_trusted_hosts varchar"),
    (13, "ALTER TABLE config ADD COLUMN confirm_prefix_skip_tokens varchar"),
    (14, "ALTER TABLE config ADD COLUMN confirm_prefix_tools varchar"),
    (15, "ALTER TABLE config ADD COLUMN audit_policy text"),
    (16, "ALTER TABLE config ADD COLUMN index_include_tests boolean NOT NULL DEFAULT 0"),
    (17, "ALTER TABLE config ADD COLUMN index_ignore varchar"),
    (18, "ALTER TABLE config ADD COLUMN index_dirs varchar"),
    (19, "ALTER TABLE config ADD COLUMN client_device_id varchar"),
    (20, "ALTER TABLE config ADD COLUMN shell_cpu_percent smallint NOT NULL DEFAULT 0"),
    (21, "ALTER TABLE config ADD COLUMN open_here_enabled boolean NOT NULL DEFAULT 1"),
    (22, "ALTER TABLE ai_provide ADD COLUMN kind integer NOT NULL DEFAULT 0"),
];

#[base_app(sort = 101)]
#[derive(Clone, Service)]
pub struct DataMigrator {
    #[inject(component)]
    data_conn_factory: DataConnFactory,
}

impl DataMigrator {
    pub fn latest_version() -> i32 {
        MIGRATIONS.last().map(|(version, _)| *version).unwrap_or(0)
    }

    fn add_column(sql: &str) -> Option<(&str, &str)> {
        let rest = sql.strip_prefix("ALTER TABLE ")?;
        let (table, rest) = rest.split_once(" ADD COLUMN ")?;
        let column = rest.split_whitespace().next()?;
        Some((table, column))
    }

    async fn read_int(conn: &DatabaseConnection, sql: &str, column: &str) -> Result<i32> {
        let row = conn
            .query_one(Statement::from_string(DatabaseBackend::Sqlite, sql.to_string()))
            .await
            .with_context(|| format!("failed to execute {sql}"))?
            .ok_or_else(|| anyhow!("{sql} returned no rows"))?;
        Ok(row.try_get::<i32>("", column)?)
    }

    async fn set_version(conn: &DatabaseConnection, version: i32) -> Result<()> {
        conn.execute_unprepared(&format!("PRAGMA user_version = {version}")).await.with_context(|| format!("failed to write database version {version}"))?;
        Ok(())
    }

    pub async fn run_migrations(conn: &DatabaseConnection) -> Result<()> {
        let latest = Self::latest_version();
        let tables = Self::read_int(conn, "SELECT COUNT(*) AS total FROM sqlite_master WHERE type = 'table'", "total").await?;
        if tables == 0 {
            Self::set_version(conn, latest).await?;
            info!("[migrate] 新库, 建表直接按当前实体定义走, 标记版本 {}", latest);
            return Ok(());
        }
        let current = Self::read_int(conn, "PRAGMA user_version", "user_version").await?;
        if current >= latest {
            return Ok(());
        }
        info!("[migrate] 旧库版本 {}, 目标版本 {}", current, latest);
        for (version, sql) in MIGRATIONS {
            if *version <= current {
                continue;
            }
            if let Some((table, column)) = Self::add_column(sql) {
                let exists = Self::read_int(conn, &format!("SELECT COUNT(*) AS total FROM pragma_table_info('{table}') WHERE name = '{column}'"), "total").await?;
                if exists > 0 {
                    Self::set_version(conn, *version).await?;
                    info!("[migrate] 已升到版本 {}", version);
                    continue;
                }
            }
            conn.execute_unprepared(sql).await.with_context(|| format!("migration to version {version} failed: {sql}"))?;
            Self::set_version(conn, *version).await?;
            info!("[migrate] 已升到版本 {}", version);
        }
        Ok(())
    }
}

#[async_trait]
impl BaseApp for DataMigrator {
    fn phase(&self) -> i16 {
        15
    }

    fn name(&self) -> &'static str {
        "DataMigrator"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Self::run_migrations(&self.data_conn_factory.conn()).await.unwrap_or_else(|error| panic!("database migration failed, refusing to start with an incomplete schema: {error:?}"));
        Ok(())
    }
}
