use std::time::Duration;
use crate::util::id_factory::uuid;
use crate::model::device::DEVICE_LOCAL_ID;
use summer::plugin::component::ComponentRef;
use crate::service::service_base::BaseService;
use crate::service::service_device::DeviceService;
use crate::service::service_config_ssh::ConfigSshService;
use crate::module::common::context::context_base::WindowSize;
use crate::module::ssh_shell::client_ssh_shell::ClientSshShell;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::context::context_ssh_shell::ContextSshShell;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;

pub const SSH_SHELL_EXEC_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn resolve_ssh_shell_context(device_id: &str, device_service: &DeviceService, config_ssh_service: &ConfigSshService) -> Result<ContextSshShell, String> {
    if device_id.is_empty() {
        return Err("device_id is empty".to_string());
    }
    if device_id == DEVICE_LOCAL_ID {
        return Err("cannot run remote SSH against the local device".to_string());
    }
    let device = device_service.find_by_id(device_id.to_string()).await.map_err(|error| error.to_string())?.ok_or_else(|| format!("device not found: {}", device_id))?;
    let config = config_ssh_service.get_by_device_id(device_id.to_string()).ok_or_else(|| format!("config_ssh not found for device: {}", device_id))?;
    Ok(ContextSshShell { size: WindowSize::default(), config, device })
}

pub async fn ssh_shell_open_exec_close(
    command: String,
    exec_timeout: Duration,
    context: &ContextSshShell,
    on_chunk: Option<ShellChunkCallback>,
    factory: ComponentRef<ClientSessionFactory>,
) -> Result<(i32, String), String> {
    let client = ClientSshShell::new(uuid(), factory);
    if let Err(error) = client.open(context, Some(true)).await {
        return Err(error.to_string());
    }
    let result = client.exec(command, exec_timeout, on_chunk).await.map_err(|error| error.to_string());
    let _ = client.close().await;
    result
}
