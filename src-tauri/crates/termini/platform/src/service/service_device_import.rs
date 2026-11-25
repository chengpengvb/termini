use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::model::device::Device;
use crate::base::base_app::BaseApp;
use crate::model::config_ssh::ConfigSsh;
use crate::model::req::req_device::ReqDevice;
use crate::service::service_base::BaseService;
use crate::model::config_ssh::SSH_DEFAULT_PORT;
use crate::service::service_device::DeviceService;
use crate::model::config_term_base::ConfigTermBase;
use crate::service::service_config_ssh::ConfigSshService;
use crate::model::resp::resp_device_import::{DeviceImportItem, RespDeviceImportApply};
use crate::model::req::req_device_import::{ReqDeviceImportApply, ReqDeviceImportPreview, IMPORT_SOURCE_PUTTY, IMPORT_SOURCE_SSH_CONFIG, IMPORT_SOURCE_TERMIUS, IMPORT_SOURCE_XSHELL};

#[base_app(sort = 121)]
#[derive(Clone, Service)]
pub struct DeviceImportService {
    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_ssh_service: ConfigSshService,
}

#[async_trait]
impl BaseApp for DeviceImportService {
    fn name(&self) -> &'static str {
        "DeviceImportService"
    }
}

impl DeviceImportService {
    pub async fn preview(&self, req: ReqDeviceImportPreview) -> anyhow::Result<Vec<DeviceImportItem>> {
        let mut items = match req.source {
            IMPORT_SOURCE_SSH_CONFIG => {
                let content = if req.content.trim().is_empty() { Self::read_local_ssh_config()? } else { req.content };
                Self::parse_ssh_config(&content)
            }
            IMPORT_SOURCE_PUTTY => Self::read_putty_sessions()?,
            IMPORT_SOURCE_XSHELL => Self::read_xshell_sessions()?,
            IMPORT_SOURCE_TERMIUS => {
                if req.content.trim().is_empty() {
                    anyhow::bail!("paste the CSV content exported from Termius first");
                }
                Self::parse_termius_csv(&req.content)
            }
            other => anyhow::bail!("unknown import source: {}", other),
        };
        let existing = self.existing_endpoints().await?;
        for item in &mut items {
            item.exists = existing.contains(&Self::endpoint_key(&item.host, item.port));
        }
        Ok(items)
    }

    pub async fn apply(&self, req: ReqDeviceImportApply) -> anyhow::Result<RespDeviceImportApply> {
        let existing = self.existing_endpoints().await?;
        let mut imported = 0;
        let mut skipped = 0;
        let mut imported_ids: Vec<(DeviceImportItem, String)> = Vec::new();
        for item in req.items {
            if existing.contains(&Self::endpoint_key(&item.host, item.port)) {
                skipped += 1;
                continue;
            }
            let saved = self.device_service.save_with_config(Self::build_req_device(&item)?).await?;
            imported_ids.push((item, saved.device().id.clone()));
            imported += 1;
        }
        let jump_linked = self.link_jumps(&imported_ids).await?;
        info!("[device] 导入完成 imported={} skipped={} jump_linked={}", imported, skipped, jump_linked);
        Ok(RespDeviceImportApply { imported, skipped, jump_linked })
    }

    fn build_req_device(item: &DeviceImportItem) -> anyhow::Result<ReqDevice> {
        let mut device = Device::default();
        device.name = item.name.clone();
        device.host = item.host.clone();

        let mut config_ssh = ConfigSsh::default();
        let base = ConfigTermBase { port: item.port, username: item.username.clone(), ..ConfigTermBase::default() };
        config_ssh.set_base(base)?;
        Ok(ReqDevice { device, group_id: None, config_ssh: Some(config_ssh), config_rdp: None, config_ftp: None, config_telnet: None, config_snmp: None })
    }

    async fn link_jumps(&self, imported: &[(DeviceImportItem, String)]) -> anyhow::Result<i32> {
        let mut linked = 0;
        let with_jump: Vec<&(DeviceImportItem, String)> = imported.iter().filter(|(item, _)| item.jump_name.as_deref().is_some_and(|name| !name.trim().is_empty())).collect();
        if with_jump.is_empty() {
            return Ok(0);
        }
        let devices = self.device_service.find_all().await?;
        for (item, device_id) in with_jump {
            let jump_name = item.jump_name.as_deref().unwrap_or_default().trim();
            let Some(target) = devices.iter().find(|ptr| {
                let device = ptr.read();
                device.name == jump_name || device.host == jump_name
            }) else {
                continue;
            };
            let target_id = target.read().id.clone();
            if target_id == *device_id {
                continue;
            }
            let Some(config_ptr) = self.config_ssh_service.get_by_device_id(device_id.clone()) else {
                continue;
            };
            let mut config = config_ptr.read().clone();
            config.jump_device_id = Some(target_id);
            self.config_ssh_service.update(config).await?;
            linked += 1;
        }
        Ok(linked)
    }

    async fn existing_endpoints(&self) -> anyhow::Result<std::collections::HashSet<String>> {
        let devices = self.device_service.find_all().await?;
        let config_map = self.config_ssh_service.get_device_id_map();
        let mut endpoints = std::collections::HashSet::new();
        for device_ptr in devices {
            let device = device_ptr.read();
            if device.host.trim().is_empty() {
                continue;
            }
            let port = config_map.get(device.id.as_str()).and_then(|ptr| ptr.read().get_base().ok()).map(|base| base.port).unwrap_or(0);
            let port = if port > 0 { port } else { SSH_DEFAULT_PORT };
            endpoints.insert(Self::endpoint_key(&device.host, port));
        }
        Ok(endpoints)
    }

    fn endpoint_key(host: &str, port: u16) -> String {
        format!("{}:{}", host.trim().to_lowercase(), port)
    }

    fn read_local_ssh_config() -> anyhow::Result<String> {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))?;
        let path = std::path::Path::new(&home).join(".ssh").join("config");
        if !path.exists() {
            anyhow::bail!("{} not found on this machine", path.display());
        }
        Ok(std::fs::read_to_string(path)?)
    }

    fn parse_ssh_config(content: &str) -> Vec<DeviceImportItem> {
        let mut items: Vec<DeviceImportItem> = Vec::new();
        let mut current: Option<DeviceImportItem> = None;
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = Self::split_directive(line) else {
                continue;
            };
            if key.eq_ignore_ascii_case("host") {
                if let Some(item) = current.take() {
                    items.push(item);
                }
                let alias = value.split_whitespace().next().unwrap_or_default().to_string();
                if alias.is_empty() || alias.contains('*') || alias.contains('?') {
                    current = None;
                    continue;
                }
                current = Some(DeviceImportItem { name: alias.clone(), host: alias, port: SSH_DEFAULT_PORT, username: None, jump_name: None, exists: false });
                continue;
            }
            let Some(item) = current.as_mut() else {
                continue;
            };
            if key.eq_ignore_ascii_case("hostname") {
                item.host = value.to_string();
            } else if key.eq_ignore_ascii_case("port") {
                if let Ok(port) = value.parse::<u16>() {
                    item.port = port;
                }
            } else if key.eq_ignore_ascii_case("user") {
                item.username = Some(value.to_string());
            } else if key.eq_ignore_ascii_case("proxyjump") {
                let first_hop = value.split(',').next().unwrap_or_default().trim();
                let host_part = first_hop.rsplit('@').next().unwrap_or_default();
                let host_only = host_part.split(':').next().unwrap_or_default().trim();
                if !host_only.is_empty() && !first_hop.eq_ignore_ascii_case("none") {
                    item.jump_name = Some(host_only.to_string());
                }
            }
        }
        if let Some(item) = current.take() {
            items.push(item);
        }
        items
    }

    fn split_directive(line: &str) -> Option<(&str, &str)> {
        let (key, value) = match line.split_once('=') {
            Some((key, value)) => (key, value),
            None => line.split_once(char::is_whitespace)?,
        };
        let value = value.trim().trim_matches('"');
        if value.is_empty() {
            return None;
        }
        Some((key.trim(), value))
    }

    fn parse_termius_csv(content: &str) -> Vec<DeviceImportItem> {
        let mut rows = content.lines().filter(|line| !line.trim().is_empty());
        let Some(header) = rows.next() else {
            return Vec::new();
        };
        let columns: Vec<String> = Self::split_csv_row(header.trim_start_matches('\u{feff}')).iter().map(|item| item.trim().to_lowercase()).collect();
        let column_index = |name: &str| columns.iter().position(|item| item.as_str() == name);
        let Some(host_index) = column_index("hostname/ip") else {
            return Vec::new();
        };
        let port_index = column_index("port");
        let label_index = column_index("label");
        let user_index = column_index("username");
        let protocol_index = column_index("protocol");

        let mut items: Vec<DeviceImportItem> = Vec::new();
        for row in rows {
            let cells = Self::split_csv_row(row);
            let cell = |index: Option<usize>| index.and_then(|index| cells.get(index)).map(|item| item.trim()).filter(|item| !item.is_empty());
            let Some(host) = cells.get(host_index).map(|item| item.trim()).filter(|item| !item.is_empty()) else {
                continue;
            };
            if let Some(protocol) = cell(protocol_index) {
                if !protocol.eq_ignore_ascii_case("ssh") {
                    continue;
                }
            }
            let port = match cell(port_index).and_then(|item| item.parse::<u16>().ok()) {
                Some(port) => port,
                None => SSH_DEFAULT_PORT,
            };
            let name = cell(label_index).unwrap_or(host).to_string();
            items.push(DeviceImportItem { name, host: host.to_string(), port, username: cell(user_index).map(|item| item.to_string()), jump_name: None, exists: false });
        }
        items
    }

    fn split_csv_row(row: &str) -> Vec<String> {
        let mut cells: Vec<String> = Vec::new();
        let mut cell = String::new();
        let mut quoted = false;
        let mut chars = row.chars().peekable();
        while let Some(current) = chars.next() {
            if quoted {
                if current != '"' {
                    cell.push(current);
                } else if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                }
                continue;
            }
            match current {
                '"' => quoted = true,
                ',' => cells.push(std::mem::take(&mut cell)),
                _ => cell.push(current),
            }
        }
        cells.push(cell);
        cells
    }

    #[cfg(target_os = "windows")]
    fn parse_xshell_session(name: &str, content: &str) -> Option<DeviceImportItem> {
        let mut section = String::new();
        let mut host = String::new();
        let mut protocol = String::new();
        let mut port = SSH_DEFAULT_PORT;
        let mut username: Option<String> = None;
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(inner) = line.strip_prefix('[').and_then(|item| item.strip_suffix(']')) {
                section = inner.trim().to_uppercase();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            if section == "CONNECTION" {
                if key.eq_ignore_ascii_case("host") {
                    host = value.to_string();
                } else if key.eq_ignore_ascii_case("protocol") {
                    protocol = value.to_string();
                } else if key.eq_ignore_ascii_case("port") {
                    if let Ok(parsed) = value.parse::<u16>() {
                        port = parsed;
                    }
                }
            } else if section == "CONNECTION:AUTHENTICATION" && key.eq_ignore_ascii_case("username") {
                username = Some(value.to_string());
            }
        }
        if host.is_empty() || !protocol.eq_ignore_ascii_case("ssh") {
            return None;
        }
        Some(DeviceImportItem { name: name.to_string(), host, port, username, jump_name: None, exists: false })
    }

    #[cfg(target_os = "windows")]
    fn read_xshell_sessions() -> anyhow::Result<Vec<DeviceImportItem>> {
        let roots = Self::xshell_session_roots();
        if roots.is_empty() {
            anyhow::bail!("Xshell sessions directory not found on this machine");
        }
        let mut items = Vec::new();
        for root in &roots {
            Self::collect_xshell_dir(root, &mut items);
        }
        Ok(items)
    }

    #[cfg(target_os = "windows")]
    fn xshell_session_roots() -> Vec<std::path::PathBuf> {
        let Ok(home) = std::env::var("USERPROFILE") else {
            return Vec::new();
        };
        let documents = std::path::Path::new(&home).join("Documents");
        let mut roots: Vec<std::path::PathBuf> = Vec::new();
        let legacy = documents.join("NetSarang").join("Xshell").join("Sessions");
        if legacy.is_dir() {
            roots.push(legacy);
        }
        let Ok(entries) = std::fs::read_dir(documents.join("NetSarang Computer")) else {
            return roots;
        };
        for entry in entries.flatten() {
            let sessions = entry.path().join("Xshell").join("Sessions");
            if sessions.is_dir() {
                roots.push(sessions);
            }
        }
        roots
    }

    #[cfg(target_os = "windows")]
    fn collect_xshell_dir(dir: &std::path::Path, items: &mut Vec<DeviceImportItem>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::collect_xshell_dir(&path, items);
                continue;
            }
            let Some(extension) = path.extension().and_then(|item| item.to_str()) else {
                continue;
            };
            if !extension.eq_ignore_ascii_case("xsh") {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let content = String::from_utf8(bytes).unwrap_or_else(|error| Self::ansi_bytes_to_string(error.as_bytes()));
            let name = path.file_stem().and_then(|item| item.to_str()).unwrap_or_default().to_string();
            if let Some(item) = Self::parse_xshell_session(&name, &content) {
                items.push(item);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn read_xshell_sessions() -> anyhow::Result<Vec<DeviceImportItem>> {
        anyhow::bail!("Xshell import is only supported on Windows")
    }

    #[cfg(target_os = "windows")]
    fn read_putty_sessions() -> anyhow::Result<Vec<DeviceImportItem>> {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let sessions = match hkcu.open_subkey("Software\\SimonTatham\\PuTTY\\Sessions") {
            Ok(key) => key,
            Err(_) => anyhow::bail!("no PuTTY sessions found in the local registry"),
        };
        let mut items = Vec::new();
        for session_name in sessions.enum_keys().flatten() {
            let Ok(session) = sessions.open_subkey(&session_name) else {
                continue;
            };
            let protocol: String = session.get_value("Protocol").unwrap_or_default();
            if !protocol.eq_ignore_ascii_case("ssh") {
                continue;
            }
            let host_raw: String = session.get_value("HostName").unwrap_or_default();
            if host_raw.trim().is_empty() {
                continue;
            }
            let port: u32 = session.get_value("PortNumber").unwrap_or(SSH_DEFAULT_PORT as u32);
            let username_value: String = session.get_value("UserName").unwrap_or_default();
            let (user_from_host, host) = match host_raw.split_once('@') {
                Some((user, host)) => (Some(user.to_string()), host.to_string()),
                None => (None, host_raw),
            };
            let username = if username_value.trim().is_empty() { user_from_host } else { Some(username_value) };
            items.push(DeviceImportItem { name: Self::decode_putty_name(&session_name), host, port: port.min(u16::MAX as u32) as u16, username, jump_name: None, exists: false });
        }
        Ok(items)
    }

    #[cfg(not(target_os = "windows"))]
    fn read_putty_sessions() -> anyhow::Result<Vec<DeviceImportItem>> {
        anyhow::bail!("PuTTY import is only supported on Windows")
    }

    #[cfg(target_os = "windows")]
    fn decode_putty_name(raw: &str) -> String {
        let bytes = raw.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' && index + 2 < bytes.len() {
                if let Ok(byte) = u8::from_str_radix(std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or_default(), 16) {
                    out.push(byte);
                    index += 3;
                    continue;
                }
            }
            out.push(bytes[index]);
            index += 1;
        }
        String::from_utf8(out).unwrap_or_else(|err| Self::ansi_bytes_to_string(err.as_bytes()))
    }

    #[cfg(target_os = "windows")]
    fn ansi_bytes_to_string(bytes: &[u8]) -> String {
        use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_ACP};
        unsafe {
            let needed = MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0);
            if needed <= 0 {
                return String::from_utf8_lossy(bytes).to_string();
            }
            let mut wide = vec![0u16; needed as usize];
            MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, wide.as_mut_ptr(), needed);
            String::from_utf16_lossy(&wide)
        }
    }
}
