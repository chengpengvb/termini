use std::time::{Duration, Instant};
use crate::model::device::DeviceOsType;

struct DeviceTypeRule {
    os_type: DeviceOsType,
    patterns: &'static [&'static str],
}

const DEVICE_TYPE_RULES: &[DeviceTypeRule] = &[
    DeviceTypeRule { os_type: DeviceOsType::Ubuntu, patterns: &["welcome to ubuntu", "ubuntu linux", "id=ubuntu"] },
    DeviceTypeRule { os_type: DeviceOsType::Debian, patterns: &["debian gnu/linux", "id=debian"] },
    DeviceTypeRule { os_type: DeviceOsType::CentOS, patterns: &["centos linux", "id=centos"] },
    DeviceTypeRule { os_type: DeviceOsType::Rhel, patterns: &["red hat enterprise linux", "id=rhel", "id=\"rhel\""] },
    DeviceTypeRule { os_type: DeviceOsType::Rocky, patterns: &["rocky linux", "id=rocky"] },
    DeviceTypeRule { os_type: DeviceOsType::AlmaLinux, patterns: &["almalinux", "id=almalinux"] },
    DeviceTypeRule { os_type: DeviceOsType::Arch, patterns: &["arch linux", "id=arch"] },
    DeviceTypeRule { os_type: DeviceOsType::OpenSuse, patterns: &["opensuse", "suse linux", "id=opensuse", "id=\"opensuse"] },
    DeviceTypeRule { os_type: DeviceOsType::Fedora, patterns: &["fedora linux", "id=fedora"] },
    DeviceTypeRule { os_type: DeviceOsType::MacOS, patterns: &["darwin", "macos", "mac os x"] },
    DeviceTypeRule { os_type: DeviceOsType::Windows, patterns: &["microsoft windows", "windows server", "powershell"] },
    DeviceTypeRule { os_type: DeviceOsType::FreeBSD, patterns: &["freebsd"] },
    DeviceTypeRule { os_type: DeviceOsType::CiscoIos, patterns: &["cisco ios software", "cisco internetwork operating system", "ios-xe software", "cisco adaptive security appliance"] },
    DeviceTypeRule { os_type: DeviceOsType::HuaweiVrp, patterns: &["huawei versatile routing platform", "vrp (r) software", "vrp software", "huawei technologies co."] },
    DeviceTypeRule { os_type: DeviceOsType::H3cComware, patterns: &["comware software", "h3c comware", "hpe comware", "h3c technologies"] },
    DeviceTypeRule { os_type: DeviceOsType::RuijieOs, patterns: &["ruijie networks", "rgos"] },
    DeviceTypeRule { os_type: DeviceOsType::ZteZxros, patterns: &["zxros", "zte corporation"] },
    DeviceTypeRule {
        os_type: DeviceOsType::Linux,
        patterns: &[
            "gnu/linux",
            "linux",
            "amazon linux",
            "amazon linux 2",
            "amazon linux 2023",
            "alibaba cloud elastic compute service",
            "welcome to alibaba cloud elastic compute service",
            "huawei cloud elastic cloud server",
            "elastic cloud server",
            "tencent cloud cvm",
            "tencentcloud cvm",
            "cloud virtual machine",
        ],
    },
];

const BANNER_BUFFER_CAP: usize = 8 * 1024;
const DETECT_WINDOW: Duration = Duration::from_secs(5);

pub fn resolve_disable_paging_command(os_type: DeviceOsType) -> Option<&'static str> {
    match os_type {
        DeviceOsType::HuaweiVrp => Some("screen-length 0 temporary"),
        DeviceOsType::H3cComware => Some("screen-length disable"),
        DeviceOsType::CiscoIos | DeviceOsType::RuijieOs | DeviceOsType::ZteZxros => Some("terminal length 0"),
        _ => None,
    }
}

pub fn match_device_os_from_banner(input: &[u8]) -> Option<DeviceOsType> {
    let text = String::from_utf8_lossy(input).to_lowercase();
    for rule in DEVICE_TYPE_RULES {
        if rule.patterns.iter().any(|pattern| text.contains(pattern)) {
            return Some(rule.os_type);
        }
    }
    None
}

pub struct DeviceOsDetector {
    finished: bool,
    buffer: Vec<u8>,
    deadline: Option<Instant>,
}

impl DeviceOsDetector {
    pub fn new() -> Self {
        Self { finished: false, buffer: Vec::new(), deadline: None }
    }

    pub fn start(&mut self) {
        if self.finished || self.deadline.is_some() {
            return;
        }
        self.deadline = Some(Instant::now() + DETECT_WINDOW);
    }

    pub fn observe(&mut self, data: &[u8]) -> Option<DeviceOsType> {
        if self.finished {
            return None;
        }
        let Some(deadline) = self.deadline else {
            return None;
        };
        let remaining = BANNER_BUFFER_CAP.saturating_sub(self.buffer.len());
        self.buffer.extend_from_slice(&data[..data.len().min(remaining)]);
        let matched = match_device_os_from_banner(&self.buffer);
        if matched.is_some() || Instant::now() >= deadline || self.buffer.len() >= BANNER_BUFFER_CAP {
            self.finished = true;
            self.buffer = Vec::new();
        }
        matched
    }
}
