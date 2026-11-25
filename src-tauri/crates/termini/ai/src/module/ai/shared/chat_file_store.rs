use std::path::PathBuf;
use summer::plugin::Service;
use anyhow::{anyhow, Context, Result};
use crate::module::ai::shared::ChatSessionCacheManager;

pub const CHAT_FILES_DIR: &str = "chat";
pub const CHAT_TEXTS_DIR: &str = "texts";
pub const CHAT_IMAGES_DIR: &str = "images";
pub const CHAT_SCRIPTS_DIR: &str = "scripts";
pub const CHAT_OUTPUTS_DIR: &str = "outputs";
#[cfg(not(target_os = "ios"))]
const DATA_DIR_ENV: &str = "TERMINI_DATA_DIR";

#[derive(Clone, Service)]
pub struct ChatFileStore {
    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

impl ChatFileStore {
    pub fn root() -> Result<PathBuf> {
        #[cfg(target_os = "ios")]
        {
            let dir = termini_platform::util::workspace::deck_dir()?;
            std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
            Ok(dir)
        }
        #[cfg(not(target_os = "ios"))]
        {
            let data_dir = std::env::var(DATA_DIR_ENV).context("TERMINI_DATA_DIR not set")?;
            Ok(PathBuf::from(data_dir).join(CHAT_FILES_DIR))
        }
    }

    pub fn chat_dir(chat_id: &str) -> Result<PathBuf> {
        if chat_id.trim().is_empty() {
            return Err(anyhow!("chat_id is empty"));
        }
        Ok(Self::root()?.join(chat_id))
    }

    pub async fn ensure_dir(&self, chat_id: &str, sub: &str) -> Result<PathBuf> {
        let dir = Self::chat_dir(chat_id)?.join(sub);
        tokio::fs::create_dir_all(&dir).await.with_context(|| format!("create {}", dir.display()))?;
        Ok(dir)
    }

    pub async fn ensure_dir_for_req(&self, req_id: &str, sub: &str) -> Result<PathBuf> {
        let chat_id = self.chat_session_cache_manager.chat_id(req_id).ok_or_else(|| anyhow!("chat not found for req_id {}", req_id))?;
        self.ensure_dir(&chat_id, sub).await
    }

    pub async fn remove_chat(&self, chat_id: &str) -> Result<()> {
        let dir = Self::chat_dir(chat_id)?;
        if !dir.exists() {
            return Ok(());
        }
        tokio::fs::remove_dir_all(&dir).await.with_context(|| format!("remove {}", dir.display()))
    }
}
