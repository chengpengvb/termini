use std::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::util::id_factory::uuid;
use summer::tracing::{error, info};
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_device::DeviceService;
use crate::service::service_ssh_shell::SshShellService;
use crate::model::dto::ssh_identity_maps::SshIdentityMaps;
use crate::util::ssh_shell::ssh_shell_const::{IDENTITY_SECTION_SEPARATOR, READ_IDENTITY_COMMAND};

const IDENTITY_EXEC_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Service)]
pub struct SftpIdentityService {
    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    ssh_shell_service: SshShellService,
}

#[async_trait]
impl BaseApp for SftpIdentityService {
    fn name(&self) -> &'static str {
        "SshIdentityService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SftpIdentityService {
    pub async fn load_identity_maps(&self, device_id: String) -> SshIdentityMaps {
        let local_device = self.device_service.select_local_device();
        if device_id.is_empty() || device_id == local_device.read().id {
            return SshIdentityMaps::default();
        }
        let session_id = uuid();
        if let Err(e) = self.ssh_shell_service.open(session_id.clone(), device_id.clone()).await {
            error!("[sftp] 加载身份映射打开会话失败 device_id: {}, error: {}", device_id, e);
            return SshIdentityMaps::default();
        };
        let maps = match self.ssh_shell_service.exec(&session_id, READ_IDENTITY_COMMAND.to_string(), IDENTITY_EXEC_TIMEOUT).await {
            Ok((_exit_code, output)) => Self::parse_identity_output(output),
            Err(e) => {
                error!("[sftp] 加载身份映射执行命令失败 device_id: {}, error: {}", device_id, e);
                SshIdentityMaps::default()
            }
        };
        if let Err(e) = self.ssh_shell_service.close(&session_id).await {
            error!("[sftp] 加载身份映射关闭会话失败 device_id: {}, error: {}", device_id, e);
        }
        info!("[sftp] 加载身份映射完成 device_id: {}, uid: {}, gid: {}", device_id, maps.uid_to_name.len(), maps.gid_to_name.len());
        maps
    }

    fn parse_identity_output(output: String) -> SshIdentityMaps {
        let Some((uid_block, gid_block)) = output.split_once(IDENTITY_SECTION_SEPARATOR) else {
            return SshIdentityMaps::default();
        };
        SshIdentityMaps { uid_to_name: Self::parse_id_name_block(uid_block), gid_to_name: Self::parse_id_name_block(gid_block) }
    }

    fn parse_id_name_block(block: &str) -> HashMap<u32, String> {
        block.lines().filter_map(Self::parse_id_name_line).collect()
    }

    fn parse_id_name_line(line: &str) -> Option<(u32, String)> {
        let (id_text, name) = line.trim().split_once(':')?;
        let id = id_text.parse::<u32>().ok()?;
        if name.is_empty() {
            return None;
        }
        Some((id, name.to_string()))
    }
}
