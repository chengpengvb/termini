use std::path::Path;
use sea_orm::Database;
use std::path::PathBuf;
use sea_orm::DatabaseConnection;
use termini_base::dao::data_migrator::DataMigrator;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

const OLD_CONFIG_DDL: &str = r#"CREATE TABLE "config" ( "id" varchar NOT NULL PRIMARY KEY, "local_shell_id" varchar , tavily_api_key TEXT, suggest_enabled INTEGER NOT NULL DEFAULT 1, suggest_count INTEGER NOT NULL DEFAULT 5, suggest_ai_provide_id TEXT, system_language smallint NOT NULL DEFAULT 0, job_exec_timeout_secs bigint NOT NULL DEFAULT 30, job_worker_parallelism smallint NOT NULL DEFAULT 25, job_ssh_pool_max_size smallint NOT NULL DEFAULT 15, job_ssh_pool_idle_secs bigint NOT NULL DEFAULT 15, job_discover_parallelism smallint NOT NULL DEFAULT 2, job_indicator_value_cache_size smallint NOT NULL DEFAULT 36, job_alert_ai_provide_id varchar, job_discover_ai_provide_id varchar, redis_db smallint NOT NULL DEFAULT 0, redis_port integer NOT NULL DEFAULT 0, redis_host varchar NOT NULL DEFAULT '', redis_enabled boolean NOT NULL DEFAULT 0, redis_username varchar, redis_password varchar, redis_job_indicator_value_cache_size smallint NOT NULL DEFAULT 0, redis_job_indicator_value_retention_secs bigint NOT NULL DEFAULT 0, danger_words VARCHAR NOT NULL DEFAULT '', notify_enabled INTEGER NOT NULL DEFAULT 0, notify_email TEXT, close_action smallint NOT NULL DEFAULT 0, notify_tray_enabled boolean NOT NULL DEFAULT 0, notify_webhook_enabled boolean NOT NULL DEFAULT 0, notify_webhook_kind smallint NOT NULL DEFAULT 0, notify_webhook_url varchar, notify_webhook_extra varchar)"#;

const OLD_AI_PROVIDE_DDL: &str = r#"CREATE TABLE "ai_provide" ( "id" varchar NOT NULL PRIMARY KEY, url varchar NOT NULL, model varchar NOT NULL, "key" varchar, provider smallint, max_token integer)"#;

fn db_url(path: &Path) -> String {
    let path_str = path.to_string_lossy().replace('\\', "/");
    format!("sqlite:///{path_str}?mode=rwc")
}

fn temp_db(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(name);
    if path.exists() {
        std::fs::remove_file(&path).expect("remove stale fixture db");
    }
    path
}

async fn query_i32(conn: &DatabaseConnection, sql: &str, column: &str) -> i32 {
    let row = conn.query_one(Statement::from_string(DatabaseBackend::Sqlite, sql.to_string())).await.expect(sql).unwrap_or_else(|| panic!("{sql} 没有返回行"));
    row.try_get::<i32>("", column).expect(column)
}

async fn query_text(conn: &DatabaseConnection, sql: &str, column: &str) -> String {
    let row = conn.query_one(Statement::from_string(DatabaseBackend::Sqlite, sql.to_string())).await.expect(sql).unwrap_or_else(|| panic!("{sql} 没有返回行"));
    row.try_get::<String>("", column).expect(column)
}

#[test]
fn old_schema_db_migrates_to_latest() {
    let path = temp_db("termini-test-migrator-old.db");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let conn = Database::connect(db_url(&path)).await.expect("connect fixture db");
        conn.execute_unprepared(OLD_CONFIG_DDL).await.expect("create v0 config");
        conn.execute_unprepared(OLD_AI_PROVIDE_DDL).await.expect("create v0 ai_provide");
        conn.execute_unprepared("INSERT INTO config (\"id\", danger_words) VALUES ('cfg-old-fixture', 'keepme')").await.expect("seed v0 row");

        DataMigrator::run_migrations(&conn).await.expect("migrate old db");

        assert_eq!(query_i32(&conn, "PRAGMA user_version", "user_version").await, DataMigrator::latest_version());
        assert_eq!(query_text(&conn, "SELECT danger_words FROM config WHERE \"id\" = 'cfg-old-fixture'", "danger_words").await, "keepme");
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'job_heartbeat_at'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'index_include_tests'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'index_ignore'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'index_dirs'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'client_device_id'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'shell_cpu_percent'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'open_here_enabled'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('ai_provide') WHERE name = 'vision'", "total").await, 1);

        DataMigrator::run_migrations(&conn).await.expect("rerun migrate on latest db");
        assert_eq!(query_i32(&conn, "PRAGMA user_version", "user_version").await, DataMigrator::latest_version());
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM config", "total").await, 1);
    });
}

#[test]
fn fresh_db_stamped_without_running_migrations() {
    let path = temp_db("termini-test-migrator-fresh.db");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let conn = Database::connect(db_url(&path)).await.expect("connect fresh db");

        DataMigrator::run_migrations(&conn).await.expect("migrate fresh db");

        assert_eq!(query_i32(&conn, "PRAGMA user_version", "user_version").await, DataMigrator::latest_version());
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM sqlite_master WHERE type = 'table'", "total").await, 0);
    });
}

#[test]
fn version_15_db_adds_only_missing_shell_cpu_percent() {
    let path = temp_db("termini-test-migrator-v15.db");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let conn = Database::connect(db_url(&path)).await.expect("connect v15 db");
        conn.execute_unprepared(r#"CREATE TABLE "config" ( "id" varchar NOT NULL PRIMARY KEY, "suggest_count" smallint NOT NULL, "suggest_enabled" boolean NOT NULL, "index_include_tests" boolean NOT NULL, "vision_use_cloud" boolean NOT NULL, "vision_local_ai_provide_id" varchar, "image_gen_use_cloud" boolean NOT NULL, "image_gen_local_ai_provide_id" varchar, "embedding_use_cloud" boolean NOT NULL, "embedding_local_ai_provide_id" varchar, "embedding_local_dim" integer, "assist_use_cloud" boolean NOT NULL, "assist_local_ai_provide_id" varchar, "index_ignore" varchar, "index_dirs" varchar, "system_language" smallint NOT NULL, "local_shell_id" varchar, "client_device_id" varchar, "danger_words" varchar NOT NULL, "web_fetch_trusted_hosts" varchar, "confirm_prefix_skip_tokens" varchar, "confirm_prefix_tools" varchar, "audit_policy" varchar, "job_exec_timeout_secs" bigint NOT NULL, "job_worker_parallelism" smallint NOT NULL, "job_ssh_pool_max_size" smallint NOT NULL, "job_ssh_pool_idle_secs" bigint NOT NULL, "job_discover_parallelism" smallint NOT NULL, "job_indicator_value_cache_size" smallint NOT NULL, "job_alert_ai_provide_id" varchar, "job_discover_ai_provide_id" varchar, "redis_db" smallint NOT NULL, "redis_port" integer NOT NULL, "redis_host" varchar NOT NULL, "redis_enabled" boolean NOT NULL, "redis_username" varchar, "redis_password" varchar, "redis_job_indicator_value_cache_size" smallint NOT NULL, "redis_job_indicator_value_retention_secs" bigint NOT NULL, "notify_enabled" boolean NOT NULL, "notify_email" varchar, "close_action" smallint NOT NULL, "notify_tray_enabled" boolean NOT NULL, "notify_webhook_enabled" boolean NOT NULL, "notify_webhook_kind" smallint NOT NULL, "notify_webhook_url" varchar, "notify_webhook_extra" varchar, "job_heartbeat_at" bigint NOT NULL )"#).await.expect("create v15 config");
        conn.execute_unprepared(r#"CREATE TABLE "ai_provide" ( "id" varchar NOT NULL PRIMARY KEY, "url" varchar NOT NULL, "model" varchar NOT NULL, "key" varchar, "provider" smallint, "max_token" integer, "vision" boolean NOT NULL )"#).await.expect("create v15 ai_provide");
        conn.execute_unprepared("PRAGMA user_version = 15").await.expect("stamp v15");

        DataMigrator::run_migrations(&conn).await.expect("migrate v15 db");

        assert_eq!(query_i32(&conn, "PRAGMA user_version", "user_version").await, DataMigrator::latest_version());
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'shell_cpu_percent'", "total").await, 1);
        assert_eq!(query_i32(&conn, "SELECT COUNT(*) AS total FROM pragma_table_info('config') WHERE name = 'open_here_enabled'", "total").await, 1);
    });
}
