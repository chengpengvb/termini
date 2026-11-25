use crate::model::device::Device;
use serde::{Deserialize, Serialize};
use crate::model::config_ftp::ConfigFtp;
use crate::model::config_rdp::ConfigRdp;
use crate::model::config_ssh::ConfigSsh;
use crate::model::config_snmp::ConfigSnmp;
use crate::model::config_telnet::ConfigTelnet;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqDevice {
    pub device: Device,
    pub group_id: Option<String>,
    pub config_ssh: Option<ConfigSsh>,
    pub config_rdp: Option<ConfigRdp>,
    pub config_ftp: Option<ConfigFtp>,
    pub config_snmp: Option<ConfigSnmp>,
    pub config_telnet: Option<ConfigTelnet>,
}
