use anyhow::bail;
use std::sync::Arc;
use dashmap::DashMap;
use std::sync::OnceLock;
use crate::model::device;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use serde_json::{json, Value};
use termini_macros::base_sync;
use crate::model::group::Group;
use crate::dao::dao_base::BaseDao;
use crate::service::service_ai_crud;
use crate::dao::dao_device::DeviceDao;
use crate::util::time_util::now_millis;
use crate::model::config_ftp::ConfigFtp;
use crate::model::config_rdp::ConfigRdp;
use crate::model::config_ssh::ConfigSsh;
use std::collections::{HashMap, HashSet};
use crate::model::config_snmp::ConfigSnmp;
use crate::model::dto::shared_ptr::Shared;
use crate::model::group::DEFAULT_GROUP_ID;
use crate::model::resp::resp_tree::RespTree;
use crate::model::req::req_device::ReqDevice;
use crate::model::config_telnet::ConfigTelnet;
use crate::service::service_base::BaseService;
use crate::model::group_2_device::Group2Device;
use crate::model::resp::resp_model::ResultCode;
use crate::model::resp::resp_device::RespDevice;
use crate::service::service_group::GroupService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::req::req_chat_context::ChatTargetType;
use crate::model::base_target::{AiFieldMeta, BaseTarget};
use crate::service::service_config_ftp::ConfigFtpService;
use crate::service::service_config_rdp::ConfigRdpService;
use crate::service::service_config_ssh::ConfigSshService;
use crate::service::service_config_snmp::ConfigSnmpService;
use crate::service::service_port_forward::PortForwardService;
use crate::service::service_command_allow::CommandAllowService;
use crate::service::service_config_telnet::ConfigTelnetService;
use crate::model::device::{Device, DeviceType, DEVICE_LOCAL_ID};
use crate::service::service_group_2_device::Group2DeviceService;

fn create_local_device_ptr() -> Arc<OnceLock<Shared<Device>>> {
    Arc::new(OnceLock::new())
}

#[base_sync(device::Entity, dao = "device_dao", skip_id = "000000", sync = 1)]
#[derive(Clone, Service)]
pub struct DeviceService {
    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    group_service: GroupService,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

    #[inject(component)]
    config_rdp_service: ConfigRdpService,

    #[inject(component)]
    config_ftp_service: ConfigFtpService,

    #[inject(component)]
    config_snmp_service: ConfigSnmpService,

    #[inject(component)]
    port_forward_service: PortForwardService,

    #[inject(component)]
    config_telnet_service: ConfigTelnetService,

    #[inject(component)]
    command_allow_service: CommandAllowService,

    #[inject(component)]
    group_2_device_service: Group2DeviceService,

    #[inject(func = create_local_device_ptr())]
    local_device_ptr: Arc<OnceLock<Shared<Device>>>,
}

#[async_trait]
impl BaseApp for DeviceService {
    fn name(&self) -> &'static str {
        "DeviceService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let mut local = Device::new_local();
        if let Some(stored) = self.device_dao.find_by_id(DEVICE_LOCAL_ID.to_string()).await? {
            local.profile = stored.read().profile.clone();
        }
        let _ = self.local_device_ptr.set(Shared::new(local));
        Ok(())
    }
}

#[async_trait]
impl BaseService for DeviceService {
    type Entity = device::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.device_dao.clone()))
    }

    async fn find_all(&self) -> anyhow::Result<Vec<Shared<Device>>> {
        self.find_all_registered().await
    }

    async fn find_by_id(&self, id: String) -> anyhow::Result<Option<Shared<Device>>> {
        if id == DEVICE_LOCAL_ID {
            return Ok(Some(self.select_local_device()));
        }
        self.get_dao()?.find_by_id(id).await
    }
}

impl DeviceService {
    pub fn select_local_device(&self) -> Shared<Device> {
        if let Some(ptr) = self.local_device_ptr.get() {
            return ptr.clone();
        }
        let device_ptr: Shared<Device> = Shared::new(Device::new_local());
        let _ = self.local_device_ptr.set(device_ptr.clone());
        device_ptr
    }

    pub fn local_device(&self) -> Shared<RespDevice> {
        let device = self.select_local_device();
        let resp_device = RespDevice::new(device.read().clone(), None, None, None, None, None, None);
        Shared::new(resp_device)
    }

    pub async fn select_with_config(&self, id: String) -> anyhow::Result<Option<RespDevice>> {
        let Some(device_ptr) = self.find_by_id(id).await? else {
            return Ok(None);
        };
        let device = device_ptr.read().clone();
        if device.id == DEVICE_LOCAL_ID {
            return Ok(Some(RespDevice::new(device, None, None, None, None, None, None)));
        }
        let config_ssh = match self.config_ssh_service.get_by_device_id(device.id.clone()) {
            Some(ptr) => Some(ptr.read().clone()),
            None => Some(self.ensure_config_ssh(device.id.clone()).await?),
        };
        let config_telnet = match self.config_telnet_service.get_by_device_id(device.id.clone()) {
            Some(ptr) => Some(ptr.read().clone()),
            None => Some(self.ensure_config_telnet(device.id.clone()).await?),
        };
        let config_rdp = match self.config_rdp_service.get_by_device_id(device.id.clone()) {
            Some(ptr) => Some(ptr.read().clone()),
            None => Some(self.ensure_config_rdp(device.id.clone()).await?),
        };
        let config_snmp = match self.config_snmp_service.get_by_device_id(device.id.clone()) {
            Some(ptr) => Some(ptr.read().clone()),
            None => Some(self.ensure_config_snmp(device.id.clone()).await?),
        };
        let config_ftp = match self.config_ftp_service.get_by_device_id(device.id.clone()) {
            Some(ptr) => Some(ptr.read().clone()),
            None => Some(self.ensure_config_ftp(device.id.clone()).await?),
        };
        let group_id = match self.group_2_device_service.find_by_device_id(device.id.as_str()).await {
            Some(ptr) => Some(ptr.read().group_id.clone()),
            None => DEFAULT_GROUP_ID.to_string().into(),
        };
        Ok(Some(RespDevice::new(device, config_ssh, config_telnet, config_rdp, config_snmp, config_ftp, group_id)))
    }

    async fn ensure_config_ssh(&self, device_id: String) -> anyhow::Result<ConfigSsh> {
        let cfg = ConfigSsh::default();
        let cfg = self.normalize_config_ssh(device_id.as_str(), cfg)?;
        let saved = self.config_ssh_service.insert(cfg).await?;
        let result = saved.read().clone();
        Ok(result)
    }

    async fn ensure_config_telnet(&self, device_id: String) -> anyhow::Result<ConfigTelnet> {
        let cfg = ConfigTelnet::default();
        let cfg = self.normalize_config_telnet(device_id.as_str(), cfg)?;
        let saved = self.config_telnet_service.insert(cfg).await?;
        let result = saved.read().clone();
        Ok(result)
    }

    async fn ensure_config_rdp(&self, device_id: String) -> anyhow::Result<ConfigRdp> {
        if let Some(ptr) = self.config_rdp_service.get_by_device_id(device_id.clone()) {
            return Ok(ptr.read().clone());
        }
        let cfg = ConfigRdp::default();
        let cfg = self.normalize_config_rdp(device_id.as_str(), cfg)?;
        let saved = self.config_rdp_service.insert(cfg).await?;
        let result = saved.read().clone();
        Ok(result)
    }

    async fn ensure_config_snmp(&self, device_id: String) -> anyhow::Result<ConfigSnmp> {
        let mut cfg = ConfigSnmp::default();
        cfg.device_id = device_id;
        let saved = self.config_snmp_service.insert(cfg).await?;
        let result = saved.read().clone();
        Ok(result)
    }

    async fn ensure_config_ftp(&self, device_id: String) -> anyhow::Result<ConfigFtp> {
        if let Some(ptr) = self.config_ftp_service.get_by_device_id(device_id.clone()) {
            return Ok(ptr.read().clone());
        }
        let cfg = ConfigFtp::default();
        let cfg = self.normalize_config_ftp(device_id.as_str(), cfg)?;
        let saved = self.config_ftp_service.insert(cfg).await?;
        let result = saved.read().clone();
        Ok(result)
    }

    pub async fn select_all_with_config(&self) -> anyhow::Result<Vec<RespDevice>> {
        let mut devices = self.find_all_registered().await?;
        Self::sort_devices_by_updated_at_desc(&mut devices);
        let mut resp_devices = Vec::with_capacity(devices.len());
        let config_ssh_map = self.config_ssh_service.get_device_id_map();
        let config_rdp_map = self.config_rdp_service.get_device_id_map();
        let config_ftp_map = self.config_ftp_service.get_device_id_map();
        let config_snmp_map = self.config_snmp_service.get_device_id_map();
        let config_telnet_map = self.config_telnet_service.get_device_id_map();
        let group_2_device_map = self.group_2_device_service.find_by_device_id_2_group_map().await;
        for device_ptr in devices {
            let device = device_ptr.read().clone();
            let config_ssh = config_ssh_map.get(device.id.as_str()).map(|ptr| ptr.read().clone());
            let config_rdp = config_rdp_map.get(device.id.as_str()).map(|ptr| ptr.read().clone());
            let config_ftp = config_ftp_map.get(device.id.as_str()).map(|ptr| ptr.read().clone());
            let config_snmp = config_snmp_map.get(device.id.as_str()).map(|ptr| ptr.read().clone());
            let config_telnet = config_telnet_map.get(device.id.as_str()).map(|ptr| ptr.read().clone());
            let group_id = group_2_device_map.get(device.id.as_str()).and_then(|entry| entry.value().first().map(|row| row.group_id.clone())).or_else(|| Some(DEFAULT_GROUP_ID.to_string()));
            resp_devices.push(RespDevice::new(device, config_ssh, config_telnet, config_rdp, config_snmp, config_ftp, group_id));
        }
        Ok(resp_devices)
    }

    pub async fn update_os_type(&self, device_id: String, os_type: i16) -> anyhow::Result<()> {
        let Some(device_ptr) = self.find_by_id(device_id.clone()).await? else {
            return Err(ResultCode::DeviceNotFound.into());
        };
        let mut device = device_ptr.read().clone();
        if device.os_type == os_type {
            return Ok(());
        }
        device.os_type = os_type;
        self.device_dao.update(device).await?;
        info!("[device] 设备[{}]系统类型识别为[{}].", device_id, os_type);
        Ok(())
    }

    pub async fn update_profile(&self, device_id: String, profile: String) -> anyhow::Result<()> {
        if device_id == DEVICE_LOCAL_ID {
            return self.update_local_profile(profile).await;
        }
        let Some(device_ptr) = self.find_by_id(device_id.clone()).await? else {
            return Err(ResultCode::DeviceNotFound.into());
        };
        let mut device = device_ptr.read().clone();
        device.profile = Some(profile);
        device.updated_at = now_millis();
        self.device_dao.update(device).await?;
        info!("[device] 设备[{}]档案已更新.", device_id);
        Ok(())
    }

    async fn update_local_profile(&self, profile: String) -> anyhow::Result<()> {
        let local_ptr = self.select_local_device();
        {
            let mut local = local_ptr.write();
            local.profile = Some(profile);
            local.updated_at = now_millis();
        }
        let local = local_ptr.read().clone();
        if self.device_dao.find_by_id(DEVICE_LOCAL_ID.to_string()).await?.is_some() {
            self.device_dao.update(local).await?;
        } else {
            self.device_dao.insert(local).await?;
        }
        info!("[device] 宿主机档案已更新.");
        Ok(())
    }

    async fn find_all_registered(&self) -> anyhow::Result<Vec<Shared<Device>>> {
        let devices = self.device_dao.find_all().await?;
        Ok(devices.into_iter().filter(|device| device.read().id != DEVICE_LOCAL_ID).collect())
    }

    pub async fn delete_with_config(&self, id: String) -> anyhow::Result<()> {
        if id.is_empty() {
            return Err(ResultCode::DeviceIdEmpty.into());
        }
        self.device_dao.delete_by_id(id.clone()).await?;
        self.config_ssh_service.delete_by_device_id(id.clone()).await?;
        self.port_forward_service.delete_by_device_id(id.clone()).await?;
        self.config_telnet_service.delete_by_device_id(id.clone()).await?;
        self.config_rdp_service.delete_by_device_id(id.clone()).await?;
        self.config_ftp_service.delete_by_device_id(id.clone()).await?;
        self.config_snmp_service.delete_by_device_id(id.clone()).await?;
        self.command_allow_service.delete_by_device_id(id.clone()).await?;
        info!("[device] 删除设备[{}].", id);
        Ok(())
    }

    fn normalize_config_ssh(&self, device_id: &str, mut cfg: ConfigSsh) -> anyhow::Result<ConfigSsh> {
        cfg.device_id = device_id.to_string();
        let base = cfg.get_base()?;
        cfg.set_base(base)?;
        Ok(cfg)
    }

    fn normalize_config_telnet(&self, device_id: &str, mut cfg: ConfigTelnet) -> anyhow::Result<ConfigTelnet> {
        cfg.device_id = device_id.to_string();
        let base = cfg.get_base()?;
        cfg.set_base(base)?;
        let terminal_types = cfg.get_terminal_types()?;
        cfg.set_terminal_types(terminal_types)?;
        let environment = cfg.get_environment()?;
        cfg.set_environment(environment)?;
        Ok(cfg)
    }

    fn normalize_config_rdp(&self, device_id: &str, mut cfg: ConfigRdp) -> anyhow::Result<ConfigRdp> {
        cfg.device_id = device_id.to_string();
        let base = cfg.get_base()?;
        cfg.set_base(base)?;
        Ok(cfg)
    }

    fn normalize_config_ftp(&self, device_id: &str, mut cfg: ConfigFtp) -> anyhow::Result<ConfigFtp> {
        cfg.device_id = device_id.to_string();
        let base = cfg.get_base()?;
        cfg.set_base(base)?;
        Ok(cfg)
    }

    async fn upsert_config_ssh(&self, device_id: &str, config_ssh: Option<ConfigSsh>) -> anyhow::Result<()> {
        let Some(cfg_ssh) = config_ssh else {
            return Ok(());
        };
        let mut cfg_ssh = self.normalize_config_ssh(device_id, cfg_ssh)?;

        let existing_ptr = self.config_ssh_service.get_by_device_id(device_id.to_string());
        let Some(existing_ptr) = existing_ptr else {
            self.config_ssh_service.insert(cfg_ssh).await?;
            return Ok(());
        };

        let existing_id = existing_ptr.read().id.clone();
        if !cfg_ssh.id.is_empty() && cfg_ssh.id != existing_id {
            return Err(ResultCode::ConfigIdMismatch.into());
        }
        if cfg_ssh.id.is_empty() {
            cfg_ssh.id = existing_id;
        }
        self.config_ssh_service.update(cfg_ssh).await?;
        Ok(())
    }

    async fn upsert_config_telnet(&self, device_id: &str, config_telnet: Option<ConfigTelnet>) -> anyhow::Result<()> {
        let Some(cfg_telnet) = config_telnet else {
            return Ok(());
        };
        let mut cfg_telnet = self.normalize_config_telnet(device_id, cfg_telnet)?;

        let existing_ptr = self.config_telnet_service.get_by_device_id(device_id.to_string());
        let Some(existing_ptr) = existing_ptr else {
            self.config_telnet_service.insert(cfg_telnet).await?;
            return Ok(());
        };

        let existing_id = existing_ptr.read().id.clone();
        if !cfg_telnet.id.is_empty() && cfg_telnet.id != existing_id {
            return Err(ResultCode::ConfigIdMismatch.into());
        }
        if cfg_telnet.id.is_empty() {
            cfg_telnet.id = existing_id;
        }
        self.config_telnet_service.update(cfg_telnet).await?;
        Ok(())
    }

    async fn upsert_config_snmp(&self, device_id: &str, config_snmp: Option<ConfigSnmp>) -> anyhow::Result<()> {
        let Some(mut cfg_snmp) = config_snmp else {
            return Ok(());
        };
        cfg_snmp.device_id = device_id.to_string();

        let existing_ptr = self.config_snmp_service.get_by_device_id(device_id.to_string());
        let Some(existing_ptr) = existing_ptr else {
            self.config_snmp_service.insert(cfg_snmp).await?;
            return Ok(());
        };

        let existing_id = existing_ptr.read().id.clone();
        if !cfg_snmp.id.is_empty() && cfg_snmp.id != existing_id {
            return Err(ResultCode::ConfigIdMismatch.into());
        }
        if cfg_snmp.id.is_empty() {
            cfg_snmp.id = existing_id;
        }
        self.config_snmp_service.update(cfg_snmp).await?;
        Ok(())
    }

    async fn upsert_config_rdp(&self, device_id: &str, config_rdp: Option<ConfigRdp>) -> anyhow::Result<()> {
        let Some(cfg_rdp) = config_rdp else {
            return Ok(());
        };
        let mut cfg_rdp = self.normalize_config_rdp(device_id, cfg_rdp)?;

        let existing_ptr = self.config_rdp_service.get_by_device_id(device_id.to_string());
        let Some(existing_ptr) = existing_ptr else {
            self.config_rdp_service.insert(cfg_rdp).await?;
            return Ok(());
        };

        let existing_id = existing_ptr.read().id.clone();
        if !cfg_rdp.id.is_empty() && cfg_rdp.id != existing_id {
            return Err(ResultCode::ConfigIdMismatch.into());
        }
        if cfg_rdp.id.is_empty() {
            cfg_rdp.id = existing_id;
        }
        self.config_rdp_service.update(cfg_rdp).await?;
        Ok(())
    }

    async fn upsert_config_ftp(&self, device_id: &str, config_ftp: Option<ConfigFtp>) -> anyhow::Result<()> {
        let Some(cfg_ftp) = config_ftp else {
            return Ok(());
        };
        let mut cfg_ftp = self.normalize_config_ftp(device_id, cfg_ftp)?;

        let existing_ptr = self.config_ftp_service.get_by_device_id(device_id.to_string());
        let Some(existing_ptr) = existing_ptr else {
            self.config_ftp_service.insert(cfg_ftp).await?;
            return Ok(());
        };

        let existing_id = existing_ptr.read().id.clone();
        if !cfg_ftp.id.is_empty() && cfg_ftp.id != existing_id {
            return Err(ResultCode::ConfigIdMismatch.into());
        }
        if cfg_ftp.id.is_empty() {
            cfg_ftp.id = existing_id;
        }
        self.config_ftp_service.update(cfg_ftp).await?;
        Ok(())
    }

    pub async fn save_with_config(&self, req_device: ReqDevice) -> anyhow::Result<RespDevice> {
        let ReqDevice { mut device, group_id, config_ssh, config_rdp, config_ftp, config_telnet, config_snmp } = req_device;
        device.updated_at = now_millis();

        let saved_device = self.device_dao.insert(device).await?;
        let device_id = saved_device.read().id.clone();

        let group_id = group_id.unwrap_or_else(|| DEFAULT_GROUP_ID.to_string());
        self.group_2_device_service.set_device_2_group(&group_id, &device_id).await?;

        if let Some(cfg_ssh) = config_ssh {
            let cfg_ssh = self.normalize_config_ssh(&device_id, cfg_ssh)?;
            self.config_ssh_service.insert(cfg_ssh).await?;
        }

        if let Some(cfg_telnet) = config_telnet {
            let cfg_telnet = self.normalize_config_telnet(&device_id, cfg_telnet)?;
            self.config_telnet_service.insert(cfg_telnet).await?;
        }

        if let Some(cfg_rdp) = config_rdp {
            let cfg_rdp = self.normalize_config_rdp(&device_id, cfg_rdp)?;
            self.config_rdp_service.insert(cfg_rdp).await?;
        }

        if let Some(cfg_ftp) = config_ftp {
            let cfg_ftp = self.normalize_config_ftp(&device_id, cfg_ftp)?;
            self.config_ftp_service.insert(cfg_ftp).await?;
        }

        if let Some(mut cfg_snmp) = config_snmp {
            cfg_snmp.device_id = device_id.clone();
            self.config_snmp_service.insert(cfg_snmp).await?;
        }

        self.select_with_config(device_id).await?.ok_or_else(|| anyhow::Error::from(ResultCode::DeviceNotFound))
    }

    pub async fn update_with_config(&self, req_device: ReqDevice) -> anyhow::Result<RespDevice> {
        let ReqDevice { mut device, group_id, config_ssh, config_rdp, config_ftp, config_telnet, config_snmp } = req_device;
        device.updated_at = now_millis();
        let saved_device = self.device_dao.update(device).await?;
        let device_id = saved_device.read().id.clone();
        self.upsert_config_ssh(&device_id, config_ssh).await?;
        self.upsert_config_telnet(&device_id, config_telnet).await?;
        self.upsert_config_rdp(&device_id, config_rdp).await?;
        self.upsert_config_ftp(&device_id, config_ftp).await?;
        self.upsert_config_snmp(&device_id, config_snmp).await?;

        let group_id = group_id.unwrap_or_else(|| DEFAULT_GROUP_ID.to_string());
        self.group_2_device_service.set_device_2_group(&group_id, &device_id).await?;

        self.select_with_config(device_id).await?.ok_or_else(|| anyhow::Error::from(ResultCode::DeviceNotFound))
    }

    pub async fn select_device_tree(&self) -> anyhow::Result<Vec<RespTree>> {
        let groups = self.group_service.find_all().await?;
        let mut devices = self.find_all_registered().await?;
        Self::sort_devices_by_updated_at_desc(&mut devices);
        let device_group_map = self.group_2_device_service.find_by_device_id_2_group_map().await;
        let known_group_ids: HashSet<String> = groups.iter().map(|item| item.read().id.clone()).collect();
        let mut device_children_by_group = Self::build_device_children_by_group(devices, &known_group_ids, &device_group_map)?;
        Ok(Self::build_group_trees(groups, &mut device_children_by_group))
    }

    fn sort_devices_by_updated_at_desc(devices: &mut Vec<Shared<Device>>) {
        devices.sort_by(|left, right| {
            let left_device = left.read();
            let right_device = right.read();
            let by_updated_at = right_device.updated_at.cmp(&left_device.updated_at);
            if by_updated_at.is_eq() {
                return left_device.name.cmp(&right_device.name);
            }
            by_updated_at
        });
    }

    fn build_device_children_by_group(
        devices: Vec<Shared<Device>>,
        known_group_ids: &HashSet<String>,
        device_group_map: &DashMap<String, Vec<Group2Device>>,
    ) -> anyhow::Result<HashMap<String, Vec<RespTree>>> {
        let mut children_by_group: HashMap<String, Vec<RespTree>> = HashMap::new();
        for device_ptr in devices {
            let device = device_ptr.read().clone();
            let group_ids = Self::resolve_device_group_ids(device.id.as_str(), device_group_map, known_group_ids);
            let leaf = RespTree::new(device.id.clone(), device.name.clone(), ChatTargetType::Device.as_i16(), Some(serde_json::to_string(&device)?), None);
            for group_id in group_ids {
                children_by_group.entry(group_id).or_default().push(leaf.clone());
            }
        }
        Ok(children_by_group)
    }

    fn resolve_device_group_ids(device_id: &str, device_group_map: &DashMap<String, Vec<Group2Device>>, known_group_ids: &HashSet<String>) -> Vec<String> {
        let Some(entry) = device_group_map.get(device_id) else {
            return vec![DEFAULT_GROUP_ID.to_string()];
        };
        let mut group_ids = Vec::new();
        for row in entry.value() {
            if row.group_id.is_empty() || !known_group_ids.contains(&row.group_id) {
                continue;
            }
            if group_ids.contains(&row.group_id) {
                continue;
            }
            group_ids.push(row.group_id.clone());
        }
        if group_ids.is_empty() {
            group_ids.push(DEFAULT_GROUP_ID.to_string());
        }
        group_ids
    }

    fn build_group_trees(groups: Vec<Shared<Group>>, device_children_by_group: &mut HashMap<String, Vec<RespTree>>) -> Vec<RespTree> {
        let mut trees = Vec::with_capacity(groups.len());
        for group_ptr in groups {
            let group = group_ptr.read().clone();
            trees.push(Self::build_group_node(group, device_children_by_group));
        }
        trees
    }

    fn build_group_node(group: Group, device_children_by_group: &mut HashMap<String, Vec<RespTree>>) -> RespTree {
        let children = device_children_by_group.remove(&group.id).unwrap_or_else(|| Vec::new());
        RespTree::new(group.id, group.name, ChatTargetType::Group.as_i16(), None, Some(children))
    }

    fn assert_writable_device(device: &Device) -> anyhow::Result<()> {
        if device.id == DEVICE_LOCAL_ID {
            bail!("cannot write local device {}", DEVICE_LOCAL_ID);
        }
        if device.r#type == DeviceType::Local as i8 {
            bail!("cannot write local device");
        }
        if device.name.trim().is_empty() {
            bail!("field name is empty");
        }
        if device.host.trim().is_empty() {
            bail!("field host is empty");
        }
        Ok(())
    }

    async fn ai_insert_device(&self, mut row: Value) -> anyhow::Result<Device> {
        let group_id = service_ai_crud::take_optional_string(&mut row, "group_id")?;
        let hidden = service_ai_crud::hidden_names::<Device>();
        let merged = service_ai_crud::merge_row(serde_json::to_value(Device::default())?, row, &hidden)?;
        let device: Device = serde_json::from_value(merged)?;
        Self::assert_writable_device(&device)?;
        let resp = self.save_with_config(ReqDevice { device, group_id, config_ssh: None, config_rdp: None, config_ftp: None, config_telnet: None, config_snmp: None }).await?;
        Ok(resp.device().clone())
    }

    async fn ai_update_device(&self, mut row: Value) -> anyhow::Result<Device> {
        let group_id = service_ai_crud::take_optional_string(&mut row, "group_id")?;
        let Some(id) = row.get("id").and_then(|value| value.as_str()).map(|value| value.to_string()) else {
            bail!("device update requires id");
        };
        if id == DEVICE_LOCAL_ID {
            bail!("cannot write local device {}", DEVICE_LOCAL_ID);
        }
        let Some(existing) = self.find_by_id(id.clone()).await? else {
            bail!("device not found: {}", id);
        };
        let hidden = service_ai_crud::hidden_names::<Device>();
        let current = {
            let guard = existing.read();
            serde_json::to_value(&*guard)?
        };
        let merged = service_ai_crud::merge_row(current, row, &hidden)?;
        let mut device: Device = serde_json::from_value(merged)?;
        Self::assert_writable_device(&device)?;
        device.updated_at = now_millis();
        let saved = self.device_dao.update(device).await?;
        let saved = saved.read().clone();
        if let Some(group_id) = group_id {
            self.group_2_device_service.set_device_2_group(&group_id, &saved.id).await?;
        }
        Ok(saved)
    }

    async fn ai_delete_device(&self, id: String) -> anyhow::Result<()> {
        if id.trim().is_empty() {
            bail!("id is empty");
        }
        if id == DEVICE_LOCAL_ID {
            bail!("cannot write local device {}", DEVICE_LOCAL_ID);
        }
        if self.find_by_id(id.clone()).await?.is_none() {
            bail!("device not found: {}", id);
        }
        self.delete_with_config(id).await
    }
}

#[async_trait]
impl service_ai_crud::AiCrud for DeviceService {
    fn entity(&self) -> &'static str {
        "device"
    }

    fn notify_entity_type(&self) -> i32 {
        Device::notify_entity_type()
    }

    fn fields(&self) -> &'static [AiFieldMeta] {
        Device::ai_fields()
    }

    async fn get(&self, id: String) -> anyhow::Result<Value> {
        let Some(row) = self.find_by_id(id.clone()).await? else {
            bail!("device not found: {}", id);
        };
        let model: Device = row.read().clone();
        service_ai_crud::to_ai_row(model)
    }

    async fn insert(&self, row: Value) -> anyhow::Result<Value> {
        service_ai_crud::to_ai_row(self.ai_insert_device(row).await?)
    }

    async fn update(&self, row: Value) -> anyhow::Result<Value> {
        service_ai_crud::to_ai_row(self.ai_update_device(row).await?)
    }

    async fn delete(&self, id: String) -> anyhow::Result<Value> {
        self.ai_delete_device(id).await?;
        Ok(json!({ "deleted": 1 }))
    }

    async fn insert_many(&self, rows: Vec<Value>) -> anyhow::Result<Value> {
        if rows.is_empty() {
            bail!("device insert rows is empty");
        }
        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            let device = self.ai_insert_device(row).await?;
            let Some(id) = device.get_id() else {
                bail!("device insert got empty id");
            };
            ids.push(id);
        }
        Ok(json!({ "inserted": ids.len(), "ids": ids }))
    }

    async fn delete_many(&self, ids: Vec<String>) -> anyhow::Result<Value> {
        if ids.is_empty() {
            bail!("device delete ids is empty");
        }
        let mut deleted = 0u64;
        for id in ids {
            self.ai_delete_device(id).await?;
            deleted += 1;
        }
        Ok(json!({ "deleted": deleted }))
    }

    async fn list(&self, offset: usize, limit: usize, keyword: Option<String>) -> anyhow::Result<Value> {
        let rows = self.find_all().await?;
        let mut values = Vec::with_capacity(rows.len());
        for row in rows {
            let model: Device = row.read().clone();
            values.push(service_ai_crud::to_ai_row(model)?);
        }
        Ok(service_ai_crud::page_values(values, offset, limit, keyword.as_deref()))
    }
}

inventory::submit! {
    service_ai_crud::AiCrudEntry {
        fetch: |app| {
            use summer::plugin::ComponentRegistry;
            app.try_get_component::<DeviceService>().ok().map(|comp| {
                Arc::new(comp) as Arc<dyn service_ai_crud::AiCrud>
            })
        },
    }
}
