use tar::Archive;
use std::io::Read;
use anyhow::Result;
use std::time::Duration;
use summer::tracing::info;
use flate2::read::GzDecoder;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::path::{Path, PathBuf};
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::document::external_tool::{ArchiveKind, ExternalTool, Platform};

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
static INSTALL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[base_app(sort = 764)]
#[derive(Clone, Service)]
pub struct ToolInstaller {
    #[inject(component)]
    config_service: ConfigService,
}

#[async_trait]
impl BaseApp for ToolInstaller {
    fn name(&self) -> &'static str {
        "ToolInstaller"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Ok(())
    }
}

impl ToolInstaller {
    pub fn data_dir() -> Result<PathBuf, AiToolError> {
        let raw = std::env::var("TERMINI_DATA_DIR").map_err(|_| AiToolError::Execution("TERMINI_DATA_DIR not set".to_string()))?;
        Ok(PathBuf::from(raw))
    }

    pub fn install_path(tool: ExternalTool) -> Result<PathBuf, AiToolError> {
        let platform = Platform::current();
        Ok(Self::data_dir()?.join(tool.install_dir_name()).join(tool.executable_name(platform)))
    }

    pub async fn ensure(&self, tool: ExternalTool) -> Result<PathBuf, AiToolError> {
        let target = Self::install_path(tool)?;
        if target.is_file() {
            return Ok(target);
        }
        let _guard = INSTALL_LOCK.lock().await;
        if target.is_file() {
            return Ok(target);
        }
        let platform = Platform::current();
        let asset = tool.asset_name(platform).map_err(AiToolError::Execution)?;
        let kind = ExternalTool::archive_kind(&asset).map_err(AiToolError::Execution)?;
        let mirror = self.mirror_url().await;
        let url = tool.download_url(mirror.as_deref(), &asset);
        info!("[ai] 下载 {} {} url={}", tool.name(), tool.version(), url);
        let bytes = Self::download(&url).await?;
        let executable = tool.executable_name(platform).to_string();
        let output = target.clone();
        tokio::task::spawn_blocking(move || Self::extract(kind, &bytes, &executable, &output))
            .await
            .map_err(|error| AiToolError::Execution(error.to_string()))?
            .map_err(|error| AiToolError::Execution(format!("failed to extract {}: {} (from {})", asset, error, url)))?;
        info!("[ai] {} 已安装 path={}", tool.name(), target.display());
        Ok(target)
    }

    async fn mirror_url(&self) -> Option<String> {
        let config = self.config_service.select_config().await?;
        config.tool_mirror_url.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
    }

    async fn download(url: &str) -> Result<Vec<u8>, AiToolError> {
        let client = reqwest::Client::builder().timeout(DOWNLOAD_TIMEOUT).build().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let response = client.get(url).send().await.map_err(|error| AiToolError::Execution(format!("download failed {}: {}", url, error)))?;
        if !response.status().is_success() {
            return Err(AiToolError::Execution(format!("download failed {}: status={}", url, response.status())));
        }
        let bytes = response.bytes().await.map_err(|error| AiToolError::Execution(format!("failed to read download body {}: {}", url, error)))?;
        Ok(bytes.to_vec())
    }

    fn extract(kind: ArchiveKind, bytes: &[u8], executable: &str, output: &Path) -> Result<(), String> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(|error| format!("create {}: {}", parent.display(), error))?;
        }
        let found = match kind {
            ArchiveKind::Zip => Self::extract_zip(bytes, executable, output)?,
            ArchiveKind::TarGz => Self::extract_tar(GzDecoder::new(bytes), executable, output)?,
            ArchiveKind::TarXz => {
                let mut decoded = Vec::new();
                lzma_rs::xz_decompress(&mut std::io::Cursor::new(bytes), &mut decoded).map_err(|error| format!("xz: {:?}", error))?;
                Self::extract_tar(std::io::Cursor::new(decoded), executable, output)?
            }
        };
        if !found {
            return Err(format!("archive does not contain {}", executable));
        }
        Self::mark_executable(output)?;
        Ok(())
    }

    fn extract_zip(bytes: &[u8], executable: &str, output: &Path) -> Result<bool, String> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| format!("zip: {}", error))?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| format!("zip entry: {}", error))?;
            if entry.is_dir() || !Self::file_name_matches(entry.name(), executable) {
                continue;
            }
            let mut file = std::fs::File::create(output).map_err(|error| format!("create {}: {}", output.display(), error))?;
            std::io::copy(&mut entry, &mut file).map_err(|error| format!("write {}: {}", output.display(), error))?;
            return Ok(true);
        }
        Ok(false)
    }

    fn extract_tar<R: Read>(reader: R, executable: &str, output: &Path) -> Result<bool, String> {
        let mut archive = Archive::new(reader);
        for entry in archive.entries().map_err(|error| format!("tar: {}", error))? {
            let mut entry = entry.map_err(|error| format!("tar entry: {}", error))?;
            let path = entry.path().map_err(|error| format!("tar path: {}", error))?.to_path_buf();
            let Some(name) = path.to_str() else {
                continue;
            };
            if !entry.header().entry_type().is_file() || !Self::file_name_matches(name, executable) {
                continue;
            }
            entry.unpack(output).map_err(|error| format!("unpack {}: {}", output.display(), error))?;
            return Ok(true);
        }
        Ok(false)
    }

    fn file_name_matches(entry_name: &str, executable: &str) -> bool {
        entry_name.rsplit(['/', '\\']).next().is_some_and(|name| name == executable)
    }

    #[cfg(unix)]
    fn mark_executable(path: &Path) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|error| format!("chmod {}: {}", path.display(), error))
    }

    #[cfg(not(unix))]
    fn mark_executable(_path: &Path) -> Result<(), String> {
        Ok(())
    }
}
