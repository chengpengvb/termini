use std::fs::File;
use std::io::Write;
use serde_json::json;
use tauri::AppHandle;
use serde_json::Value;
use summer::tracing::info;
use summer::plugin::Service;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use std::time::{Duration, SystemTime};
use crate::base::config::app_config::AppConfig;
use crate::service::service_config::ConfigService;
use crate::model::resp::resp_diagnostics::RespDiagnosticsExport;
use crate::service::service_impl_system_util::SystemUtilServiceImpl;

const LOG_DIR: &str = "logs";
const LOG_KEEP_DAYS: u64 = 3;
const SENSITIVE_KEY_PARTS: [&str; 6] = ["password", "key", "token", "secret", "webhook_url", "email"];

#[derive(Clone, Service)]
pub struct DiagnosticsService {
    #[inject(component)]
    handle: AppHandle,

    #[inject(component)]
    app_config: AppConfig,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    system_util_service_impl: SystemUtilServiceImpl,
}

impl DiagnosticsService {
    pub async fn export(&self, target: &str) -> Result<RespDiagnosticsExport> {
        let target_path = PathBuf::from(target);
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let manifest = self.build_manifest().await?;
        let system_info = self.system_util_service_impl.system_info().await?;
        let config = self.sanitized_config().await;
        let logs = Self::recent_logs(&self.app_config.data_dir().join(LOG_DIR))?;

        let file = File::create(&target_path).with_context(|| format!("create {}", target_path.display()))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        Self::put_json(&mut zip, "manifest.json", &manifest, options)?;
        Self::put_json(&mut zip, "system_info.json", &serde_json::to_value(&system_info)?, options)?;
        Self::put_json(&mut zip, "config.json", &config, options)?;
        for log in &logs {
            let name = format!("{}/{}", LOG_DIR, log.file_name().and_then(|name| name.to_str()).unwrap_or("unnamed.log"));
            zip.start_file(name, options)?;
            let mut source = File::open(log).with_context(|| format!("open {}", log.display()))?;
            std::io::copy(&mut source, &mut zip)?;
        }
        zip.finish()?;

        let bytes = std::fs::metadata(&target_path).map(|meta| meta.len()).unwrap_or(0);
        let files = logs.len() + 3;
        info!("[diagnostics] 诊断包已导出 path={} files={} bytes={}", target_path.display(), files, bytes);
        Ok(RespDiagnosticsExport { path: target_path.to_string_lossy().to_string(), files, bytes })
    }

    async fn build_manifest(&self) -> Result<Value> {
        let package = self.handle.package_info();
        Ok(json!({
            "app": package.name,
            "version": package.version.to_string(),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "exportedAt": chrono::Local::now().to_rfc3339(),
            "dataDir": self.app_config.data_dir().to_string_lossy(),
        }))
    }

    async fn sanitized_config(&self) -> Value {
        let Some(config) = self.config_service.select_config().await else {
            return Value::Null;
        };
        let mut value = serde_json::to_value(&config).unwrap_or(Value::Null);
        if let Value::Object(map) = &mut value {
            for (key, item) in map.iter_mut() {
                let lower = key.to_ascii_lowercase();
                if SENSITIVE_KEY_PARTS.iter().any(|part| lower.contains(part)) && !item.is_null() {
                    *item = Value::String("<redacted>".to_string());
                }
            }
        }
        value
    }

    fn recent_logs(dir: &Path) -> Result<Vec<PathBuf>> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(Vec::new());
        };
        let cutoff = SystemTime::now() - Duration::from_secs(LOG_KEEP_DAYS * 24 * 3600);
        let mut logs: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| std::fs::metadata(path).and_then(|meta| meta.modified()).map(|modified| modified >= cutoff).unwrap_or(false))
            .collect();
        logs.sort();
        Ok(logs)
    }

    fn put_json(zip: &mut zip::ZipWriter<File>, name: &str, value: &Value, options: SimpleFileOptions) -> Result<()> {
        zip.start_file(name, options)?;
        zip.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
        Ok(())
    }
}
