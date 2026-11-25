use summer::plugin::Service;
use crate::model::command_allow;
use termini_macros::base_service;
use crate::util::time_util::now_millis;
use crate::service::service_base::BaseService;
use crate::dao::dao_command_allow::CommandAllowDao;
use crate::model::command_allow::{CommandAllow, COMMAND_PREVIEW_MAX_LEN};

#[base_service(command_allow::Entity, dao = "command_allow_dao")]
#[derive(Clone, Service)]
pub struct CommandAllowService {
    #[inject(component)]
    command_allow_dao: CommandAllowDao,
}

impl CommandAllowService {
    pub fn is_allowed(&self, device_id: &str, tool_id: &str, command_hash: &str) -> bool {
        self.command_allow_dao.exists(device_id, tool_id, command_hash)
    }

    pub async fn remember(&self, device_id: &str, tool_id: &str, command_hash: &str, command_text: &str) -> anyhow::Result<()> {
        if self.command_allow_dao.exists(device_id, tool_id, command_hash) {
            return Ok(());
        }
        let preview = Self::build_preview(command_text);
        let entity = CommandAllow::build(device_id.to_string(), tool_id.to_string(), command_hash.to_string(), preview, now_millis());
        self.insert(entity).await?;
        Ok(())
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> anyhow::Result<u64> {
        self.command_allow_dao.delete_by_device_id(device_id).await
    }

    fn build_preview(command_text: &str) -> String {
        let trimmed = command_text.trim();
        if trimmed.chars().count() <= COMMAND_PREVIEW_MAX_LEN {
            return trimmed.to_string();
        }
        let head: String = trimmed.chars().take(COMMAND_PREVIEW_MAX_LEN).collect();
        format!("{}…", head)
    }
}
