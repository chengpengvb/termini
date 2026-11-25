use sea_orm::JsonValue;
use termini_macros::Redact;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use crate::model::config_term_base::ConfigTermBase;

pub const RDP_DEFAULT_PORT: u16 = 3389;
pub const COLOR_DEPTH_DEFAULT: i16 = 32;
pub const KEYBOARD_LAYOUT_DEFAULT: i32 = 0x0409;

pub const SECURITY_AUTO: i16 = 0;
pub const SECURITY_TLS: i16 = 1;
pub const SECURITY_NLA: i16 = 2;

fn default_color_depth() -> i16 {
    COLOR_DEPTH_DEFAULT
}

fn default_keyboard_layout() -> i32 {
    KEYBOARD_LAYOUT_DEFAULT
}

#[base_target(table = "config_rdp", entity_type = 14, notify = 1)]
#[derive(Clone, Serialize, Deserialize, Redact)]
pub struct Model {
    #[encrypt]
    #[redact_base = "crate::model::config_term_base::ConfigTermBase"]
    pub base: JsonValue,
    #[serde(default)]
    pub device_id: String,
    pub domain: Option<String>, //登录域，留空表示本机账户
    #[serde(default = "default_color_depth")]
    pub color_depth: i16, //会话色深，16/24/32
    #[serde(default)]
    pub security: i16, //安全层：0 自动协商 1 TLS 2 NLA
    #[serde(default)]
    pub ignore_cert: bool, //忽略服务端证书校验
    #[serde(default)]
    pub console_session: bool, //连接到控制台会话
    #[serde(default = "default_keyboard_layout")]
    pub keyboard_layout: i32, //键盘布局 LCID，默认 0x0409 美式
    pub desktop_width: Option<i16>, //固定桌面宽度，留空跟随窗口
    pub desktop_height: Option<i16>, //固定桌面高度，留空跟随窗口
    pub performance_flags: Option<i32>, //性能标志位，留空按色深推断
    pub drive_path: Option<String>, //重定向到远端的本地目录，留空不重定向
    pub aad_client_id: Option<String>, //Azure 应用注册的客户端 ID，留空不启用 Azure AD 认证
    pub aad_tenant: Option<String>, //Azure 租户 ID，留空用 common
}

impl Default for Model {
    fn default() -> Self {
        let base = ConfigTermBase { port: RDP_DEFAULT_PORT, ..ConfigTermBase::default() };
        Self { base: serde_json::to_value(base).expect("ConfigBase to_value failed"), ..Self::blank() }
    }
}

impl Model {
    fn blank() -> Self {
        Self {
            id: String::new(),
            domain: None,
            base: JsonValue::Null,
            device_id: String::new(),
            security: SECURITY_AUTO,
            ignore_cert: false,
            console_session: false,
            desktop_width: None,
            desktop_height: None,
            performance_flags: None,
            drive_path: None,
            aad_client_id: None,
            aad_tenant: None,
            color_depth: COLOR_DEPTH_DEFAULT,
            keyboard_layout: KEYBOARD_LAYOUT_DEFAULT,
        }
    }

    pub fn resolve_color_depth(&self) -> u16 {
        match self.color_depth {
            16 => 16,
            24 => 24,
            _ => 32,
        }
    }

    pub fn get_base(&self) -> anyhow::Result<ConfigTermBase> {
        Ok(serde_json::from_value(self.base.clone())?)
    }

    pub fn set_base(&mut self, base: ConfigTermBase) -> anyhow::Result<()> {
        self.base = serde_json::to_value(base)?;
        Ok(())
    }
}

pub type ConfigRdp = Model;
