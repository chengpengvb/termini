use std::path::Path;
use sea_orm::Database;
use std::path::PathBuf;
use sea_orm::EntityTrait;
use sea_orm::IntoActiveModel;
use sea_orm::ActiveModelTrait;
use sea_orm::DatabaseConnection;
use termini_core::model::device;
use termini_core::model::ai_chat;
use termini_core::model::ai_project;
use termini_core::model::config_rdp;
use termini_core::model::config_ssh;
use termini_core::model::config_telnet;
use termini_core::model::config_key_pair;
use termini_core::util::crypto_util::ENC_PREFIX;
use termini_core::model::base_target::BaseTarget;

fn app_db_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").expect("APPDATA");
    PathBuf::from(appdata).join("com.termini.www").join("app.db")
}

fn db_url(path: &Path) -> String {
    let path_str = path.to_string_lossy().replace('\\', "/");
    format!("sqlite:///{path_str}?mode=rwc")
}

fn option_len(value: &Option<String>) -> String {
    match value {
        None => "null".to_string(),
        Some(text) if text.is_empty() => "empty".to_string(),
        Some(text) if text.starts_with(ENC_PREFIX) => format!("enc len={}", text.len()),
        Some(text) => format!("plain len={}", text.len()),
    }
}

fn json_string_state(value: Option<&serde_json::Value>) -> &'static str {
    match value {
        None => "missing",
        Some(serde_json::Value::Null) => "null",
        Some(serde_json::Value::String(text)) if text.is_empty() => "empty",
        Some(serde_json::Value::String(_)) => "set",
        Some(_) => "set",
    }
}

fn describe_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::String(text) if text.starts_with(ENC_PREFIX) => format!("enc_string len={}", text.len()),
        serde_json::Value::Object(map) => {
            format!("object keys={} username={} password={}", map.len(), json_string_state(map.get("username")), json_string_state(map.get("password")))
        }
        serde_json::Value::Array(items) => format!("array len={}", items.len()),
        serde_json::Value::Bool(_) => "bool".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
    }
}

async fn repair_key_pairs(conn: &DatabaseConnection) {
    let rows = config_key_pair::Entity::find().all(conn).await.expect("load config_key_pair");
    println!("config_key_pair rows={}", rows.len());
    for mut model in rows {
        model.decrypt_sensitive();
        println!(
            "config_key_pair id={} name={} private_key={} passphrase={} certificate={}",
            model.id,
            model.name,
            option_len(&model.private_key),
            option_len(&model.passphrase),
            option_len(&model.certificate)
        );
        if let Some(text) = &model.private_key {
            if text.starts_with(ENC_PREFIX) {
                panic!("config_key_pair {} private_key 解密后仍是密文", model.id);
            }
        }
        model.encrypt_sensitive();
        model.into_active_model().reset_all().update(conn).await.expect("update config_key_pair");
    }
}

async fn repair_ssh(conn: &DatabaseConnection) {
    let rows = config_ssh::Entity::find().all(conn).await.expect("load config_ssh");
    println!("config_ssh rows={}", rows.len());
    for mut model in rows {
        model.decrypt_sensitive();
        println!("config_ssh id={} device_id={} base={} passphrase={}", model.id, model.device_id, describe_json(&model.base), option_len(&model.passphrase));
        match model.get_base() {
            Ok(_) => {}
            Err(error) => panic!("config_ssh {} get_base: {}", model.id, error),
        }
        model.encrypt_sensitive();
        model.into_active_model().reset_all().update(conn).await.expect("update config_ssh");
    }
}

async fn repair_rdp(conn: &DatabaseConnection) {
    let rows = config_rdp::Entity::find().all(conn).await.expect("load config_rdp");
    println!("config_rdp rows={}", rows.len());
    for mut model in rows {
        model.decrypt_sensitive();
        println!("config_rdp id={} device_id={} base={}", model.id, model.device_id, describe_json(&model.base));
        match model.get_base() {
            Ok(_) => {}
            Err(error) => panic!("config_rdp {} get_base: {}", model.id, error),
        }
        model.encrypt_sensitive();
        model.into_active_model().reset_all().update(conn).await.expect("update config_rdp");
    }
}

async fn repair_telnet(conn: &DatabaseConnection) {
    let rows = config_telnet::Entity::find().all(conn).await.expect("load config_telnet");
    println!("config_telnet rows={}", rows.len());
    for mut model in rows {
        model.decrypt_sensitive();
        println!("config_telnet id={} device_id={} base={}", model.id, model.device_id, describe_json(&model.base));
        match model.get_base() {
            Ok(_) => {}
            Err(error) => panic!("config_telnet {} get_base: {}", model.id, error),
        }
        model.encrypt_sensitive();
        model.into_active_model().reset_all().update(conn).await.expect("update config_telnet");
    }
}

#[test]
fn repair_encrypted_startup_fields() {
    let path = app_db_path();
    if !path.is_file() {
        panic!("app.db 不存在: {}", path.display());
    }
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let conn = Database::connect(db_url(&path)).await.expect("connect app.db");
        let device_n = device::Entity::find().all(&conn).await.expect("load device").len();
        let chat_n = ai_chat::Entity::find().all(&conn).await.expect("load ai_chat").len();
        let project_n = ai_project::Entity::find().all(&conn).await.expect("load ai_project").len();
        println!("startup rows device={} ai_chat={} ai_project={}", device_n, chat_n, project_n);
        if device_n == 0 {
            panic!("device 表为空");
        }
        if chat_n == 0 {
            panic!("ai_chat 表为空");
        }
        if project_n == 0 {
            panic!("ai_project 表为空");
        }
        repair_key_pairs(&conn).await;
        repair_ssh(&conn).await;
        repair_rdp(&conn).await;
        repair_telnet(&conn).await;
    });
}
