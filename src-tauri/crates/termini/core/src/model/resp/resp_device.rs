use serde::Serialize;
use crate::model::device::Device;
use crate::model::base_target::Redact;
use crate::model::config_ftp::ConfigFtp;
use crate::model::config_rdp::ConfigRdp;
use crate::model::config_ssh::ConfigSsh;
use crate::model::config_snmp::ConfigSnmp;
use crate::model::config_telnet::ConfigTelnet;

#[derive(Debug, Clone, Serialize)]
pub struct RespDevice {
    device: Device,
    group_id: Option<String>,
    config_ssh: Option<ConfigSsh>,
    config_rdp: Option<ConfigRdp>,
    config_ftp: Option<ConfigFtp>,
    config_snmp: Option<ConfigSnmp>,
    config_telnet: Option<ConfigTelnet>,
}

impl RespDevice {
    pub fn new(
        device: Device,
        config_ssh: Option<ConfigSsh>,
        config_telnet: Option<ConfigTelnet>,
        config_rdp: Option<ConfigRdp>,
        config_snmp: Option<ConfigSnmp>,
        config_ftp: Option<ConfigFtp>,
        group_id: Option<String>,
    ) -> Self {
        Self { device, config_ssh, config_telnet, config_rdp, config_ftp, config_snmp, group_id }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn group_id(&self) -> Option<&str> {
        self.group_id.as_deref()
    }

    pub fn redact(self) -> Self {
        Self {
            config_ssh: self.config_ssh.map(|c| c.redact()),
            config_telnet: self.config_telnet.map(|c| c.redact()),
            config_rdp: self.config_rdp.map(|c| c.redact()),
            config_ftp: self.config_ftp.map(|c| c.redact()),
            config_snmp: self.config_snmp.map(|c| c.redact()),
            ..self
        }
    }
}
