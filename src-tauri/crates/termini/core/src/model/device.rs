use termini_macros::base_target;
use crate::util::id_factory::uuid;
use serde::{Deserialize, Serialize};
use crate::util::time_util::now_millis;

pub const DEVICE_LOCAL_ID: &str = "000000";

#[repr(i8)]
pub enum DeviceType {
    Local = 0,
    Remote = 1,
}

#[repr(i16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceOsType {
    Unknown = 0,
    Ubuntu = 101,
    Debian = 102,
    CentOS = 103,
    Rhel = 104,
    Rocky = 105,
    AlmaLinux = 106,
    Arch = 107,
    OpenSuse = 108,
    Fedora = 109,
    Linux = 199,
    MacOS = 201,
    Windows = 301,
    FreeBSD = 401,
    Ios = 501,
    Android = 502,
    CiscoIos = 601,   //Cisco IOS / IOS-XE 交换机路由器
    HuaweiVrp = 602,  //华为 VRP（含 CloudEngine / S 系列）
    H3cComware = 603, //H3C / HPE Comware
    RuijieOs = 604,   //锐捷 RGOS
    ZteZxros = 605,   //中兴 ZXROS
    NetworkOs = 699,  //未细分的网络设备（交换机/路由器/防火墙）
}

impl DeviceOsType {
    pub fn as_i16(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Self {
        match value {
            101 => DeviceOsType::Ubuntu,
            102 => DeviceOsType::Debian,
            103 => DeviceOsType::CentOS,
            104 => DeviceOsType::Rhel,
            105 => DeviceOsType::Rocky,
            106 => DeviceOsType::AlmaLinux,
            107 => DeviceOsType::Arch,
            108 => DeviceOsType::OpenSuse,
            109 => DeviceOsType::Fedora,
            199 => DeviceOsType::Linux,
            201 => DeviceOsType::MacOS,
            301 => DeviceOsType::Windows,
            401 => DeviceOsType::FreeBSD,
            501 => DeviceOsType::Ios,
            502 => DeviceOsType::Android,
            601 => DeviceOsType::CiscoIos,
            602 => DeviceOsType::HuaweiVrp,
            603 => DeviceOsType::H3cComware,
            604 => DeviceOsType::RuijieOs,
            605 => DeviceOsType::ZteZxros,
            699 => DeviceOsType::NetworkOs,
            _ => DeviceOsType::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DeviceOsType::Unknown => "未识别",
            DeviceOsType::Ubuntu => "Ubuntu Linux",
            DeviceOsType::Debian => "Debian Linux",
            DeviceOsType::CentOS => "CentOS Linux",
            DeviceOsType::Rhel => "Red Hat Enterprise Linux",
            DeviceOsType::Rocky => "Rocky Linux",
            DeviceOsType::AlmaLinux => "AlmaLinux",
            DeviceOsType::Arch => "Arch Linux",
            DeviceOsType::OpenSuse => "openSUSE Linux",
            DeviceOsType::Fedora => "Fedora Linux",
            DeviceOsType::Linux => "Linux",
            DeviceOsType::MacOS => "macOS",
            DeviceOsType::Windows => "Windows",
            DeviceOsType::FreeBSD => "FreeBSD",
            DeviceOsType::Ios => "iOS",
            DeviceOsType::Android => "Android",
            DeviceOsType::CiscoIos => "Cisco IOS / IOS-XE",
            DeviceOsType::HuaweiVrp => "华为 VRP",
            DeviceOsType::H3cComware => "H3C / HPE Comware",
            DeviceOsType::RuijieOs => "锐捷 RGOS",
            DeviceOsType::ZteZxros => "中兴 ZXROS",
            DeviceOsType::NetworkOs => "网络设备（通用 CLI）",
        }
    }

    pub fn current_local() -> Self {
        match std::env::consts::OS {
            "linux" => DeviceOsType::Linux,
            "macos" => DeviceOsType::MacOS,
            "windows" => DeviceOsType::Windows,
            "freebsd" => DeviceOsType::FreeBSD,
            "ios" => DeviceOsType::Ios,
            "android" => DeviceOsType::Android,
            _ => DeviceOsType::Unknown,
        }
    }
}

#[base_target(table = "device", entity_type = 1)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub host: String,
    pub r#type: i8,
    pub os_type: i16,
    #[serde(default)]
    pub profile: Option<String>, // AI 探索沉淀的设备档案：系统角色、关键服务、暴露面
    #[serde(default, deserialize_with = "crate::model::base_target::deserialize_null_default")]
    pub updated_at: i64,
}

impl Model {
    pub fn new_local() -> Self {
        Self { id: DEVICE_LOCAL_ID.to_string(), name: "Local".into(), host: "local".into(), r#type: DeviceType::Local as i8, os_type: DeviceOsType::current_local().as_i16(), profile: None, updated_at: now_millis() }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self { id: uuid(), name: String::new(), host: String::new(), r#type: DeviceType::Remote as i8, os_type: DeviceOsType::Unknown.as_i16(), profile: None, updated_at: 0 }
    }
}

pub type Device = Model;
