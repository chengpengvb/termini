use std::time::Duration;
use crate::model::device::DEVICE_LOCAL_ID;
use crate::service::service_base::BaseService;
use crate::service::service_device::DeviceService;
use crate::module::common::context::context_base::WindowSize;
use crate::service::service_config_telnet::ConfigTelnetService;
use crate::module::common::context::context_telnet_shell::ContextTelnetShell;

pub const TELNET_SHELL_EXEC_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn resolve_telnet_shell_context(device_id: &str, device_service: &DeviceService, config_telnet_service: &ConfigTelnetService) -> Result<ContextTelnetShell, String> {
    if device_id.is_empty() {
        return Err("device_id is empty".to_string());
    }
    if device_id == DEVICE_LOCAL_ID {
        return Err("cannot run remote Telnet against the local device".to_string());
    }
    let device = device_service.find_by_id(device_id.to_string()).await.map_err(|error| error.to_string())?.ok_or_else(|| format!("device not found: {}", device_id))?;
    let config = config_telnet_service.get_by_device_id(device_id.to_string()).ok_or_else(|| format!("config_telnet not found for device: {}", device_id))?;
    Ok(ContextTelnetShell { size: WindowSize::default(), config, device })
}
