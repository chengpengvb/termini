use anyhow::anyhow;
use std::sync::Arc;
use tokio::io::WriteHalf;
use parking_lot::Mutex as SyncMutex;
use crate::module::rdp::protocol::core::gcc;
use crate::module::rdp::protocol::core::info;
use crate::module::rdp::protocol::core::nego;
use tokio::sync::{oneshot, Mutex as AsyncMutex};
use crate::module::common::util::task_hub::TaskGroup;
use crate::model::dto::clipboard::ClipboardSubscription;
use crate::module::common::client_terminal::EventHandler;
use crate::module::rdp::protocol::core::error::RdpResult;
use crate::module::rdp::protocol::core::crypt::RdpSecurity;
use crate::module::common::context::context_rdp::ContextRdp;
use crate::module::common::context::context_base::WindowSize;
use crate::module::rdp::protocol::core::transport::RdpStream;
use crate::module::rdp::protocol::core::channel::ChannelTable;
use crate::module::rdp::protocol::core::cliprdr::ClipboardState;
use crate::module::rdp::protocol::core::drdynvc::DynamicChannels;
use crate::module::rdp::protocol::core::redirection::Redirection;
use crate::module::rdp::protocol::core::logon::AutoReconnectCookie;
use crate::model::config_rdp::{ConfigRdp, SECURITY_NLA, SECURITY_TLS};
use crate::model::resp::event::resp_event_rdp::{RespEventRdp, RespRdpDesktopSize};

const MIN_DESKTOP_EDGE: u16 = 200;
const MAX_DESKTOP_EDGE: u16 = 4096;
pub(crate) const CLIENT_NAME: &str = "termini";
const DEFAULT_CONNECT_TIMEOUT_SECONDS: u64 = 15;

#[derive(Clone)]
pub struct RdpSettings {
    pub port: u16,
    pub host: String,
    pub domain: String,
    pub desktop_dpi: u16,
    pub username: String,
    pub password: String,
    pub color_depth: u16,
    pub ignore_cert: bool,
    pub desktop_width: u16,
    pub desktop_height: u16,
    pub keyboard_layout: u32,
    pub dynamic_desktop: bool,
    pub console_session: bool,
    pub performance_flags: u32,
    pub requested_protocols: u32,
    pub connect_timeout_seconds: u64,
    pub keepalive_interval_seconds: u64,
    pub aad_tenant: Option<String>,
    pub drive_path: Option<String>,
    pub redirection_session_id: u32,
    pub aad_client_id: Option<String>,
    pub routing_token: Option<Vec<u8>>,
    pub redirection_guid: Option<Vec<u8>>,
    pub redirection_password: Option<Vec<u8>>,
    pub auto_reconnect: Option<AutoReconnectCookie>,
}

impl RdpSettings {
    pub(crate) fn build(context: &ContextRdp) -> anyhow::Result<RdpSettings> {
        let config = context.config.read().clone();
        let base = config.get_base()?;
        let host = context.device.read().host.clone();
        let username = base.username.unwrap_or_default();
        let password = base.password.unwrap_or_default();
        let color_depth = config.resolve_color_depth();
        let (desktop_width, desktop_height) = Self::resolve_desktop(&config, context.size)?;
        Ok(RdpSettings {
            host,
            username,
            password,
            color_depth,
            desktop_width,
            desktop_height,
            port: base.port,
            routing_token: None,
            auto_reconnect: None,
            redirection_guid: None,
            redirection_session_id: 0,
            redirection_password: None,
            desktop_dpi: context.size.dpi,
            ignore_cert: config.ignore_cert,
            console_session: config.console_session,
            keyboard_layout: config.keyboard_layout as u32,
            domain: config.domain.clone().unwrap_or_default(),
            keepalive_interval_seconds: base.keepalive_interval_seconds,
            drive_path: config.drive_path.clone().filter(|path| !path.is_empty()),
            aad_tenant: config.aad_tenant.clone().filter(|tenant| !tenant.is_empty()),
            dynamic_desktop: config.desktop_width.is_none() && config.desktop_height.is_none(),
            aad_client_id: config.aad_client_id.clone().filter(|client_id| !client_id.is_empty()),
            requested_protocols: Self::resolve_protocols(config.security, config.aad_client_id.as_deref()),
            connect_timeout_seconds: base.connect_timeout_seconds.unwrap_or(DEFAULT_CONNECT_TIMEOUT_SECONDS).clamp(3, 120),
            performance_flags: config.performance_flags.map(|flags| flags as u32).unwrap_or_else(info::default_performance_flags),
        })
    }

    pub(crate) fn apply_redirection(&mut self, redirection: &Redirection, auto_reconnect: Option<AutoReconnectCookie>) {
        if let Some(host) = redirection.target_host() {
            self.host = host.to_string();
        }
        if let Some(username) = &redirection.username {
            self.username = username.clone();
        }
        if let Some(domain) = &redirection.domain {
            self.domain = domain.clone();
        }
        self.routing_token = redirection.load_balance_info.clone();
        self.redirection_password = redirection.password.clone();
        self.redirection_guid = redirection.redirection_guid.clone();
        self.redirection_session_id = redirection.session_id;
        self.auto_reconnect = auto_reconnect;
        if redirection.password_is_encrypted {
            self.requested_protocols |= nego::PROTOCOL_RDSTLS;
        }
    }

    fn resolve_desktop(config: &ConfigRdp, size: WindowSize) -> anyhow::Result<(u16, u16)> {
        let width = match config.desktop_width {
            Some(value) => u16::try_from(value).map_err(|_| anyhow!("invalid rdp desktop width: {}", value))?,
            None => size.pixel_width,
        };
        let height = match config.desktop_height {
            Some(value) => u16::try_from(value).map_err(|_| anyhow!("invalid rdp desktop height: {}", value))?,
            None => size.pixel_height,
        };
        if !(MIN_DESKTOP_EDGE..=MAX_DESKTOP_EDGE).contains(&width) || !(MIN_DESKTOP_EDGE..=MAX_DESKTOP_EDGE).contains(&height) {
            return Err(anyhow!("rdp desktop size out of range: {}x{}", width, height));
        }
        Ok((width & !1, height & !1))
    }

    pub fn resolve_protocols(security: i16, aad_client_id: Option<&str>) -> u32 {
        let mut protocols = match security {
            SECURITY_TLS => nego::PROTOCOL_SSL,
            SECURITY_NLA => nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID,
            _ => nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID | nego::PROTOCOL_SSL,
        };
        if security != SECURITY_TLS && aad_client_id.is_some_and(|client_id| !client_id.is_empty()) {
            protocols |= nego::PROTOCOL_RDSAAD;
        }
        protocols
    }
}

#[derive(Clone, Copy)]
pub struct RdpChannel {
    pub user_id: u16,
    pub share_id: u32,
    pub global_channel_id: u16,
    pub message_channel_id: u16,
}

pub(crate) struct RdpState {
    pub(crate) input_flags: u16,
    pub(crate) refresh_rect: bool,
    pub(crate) suppress_output: bool,
    pub(crate) max_num_monitors: u32,
    pub(crate) frame_acknowledge: bool,

    pub(crate) channel: RdpChannel,
    pub(crate) security: RdpSecurity,
    pub(crate) channels: ChannelTable,
    pub(crate) dynamic: DynamicChannels,
    pub(crate) clipboard: ClipboardState,
    pub(crate) desktop: RespRdpDesktopSize,
    pub(crate) auto_reconnect: Option<AutoReconnectCookie>,
}

pub(crate) struct RdpSession {
    pub(crate) tasks: TaskGroup,
    pub(crate) context: Arc<ContextRdp>,
    pub(crate) settings: Arc<RdpSettings>,
    pub(crate) state: Arc<SyncMutex<RdpState>>,
    pub(crate) handler: EventHandler<RespEventRdp>,
    pub(crate) writer: Arc<AsyncMutex<WriteHalf<RdpStream>>>,
    pub(crate) close_tx: SyncMutex<Option<oneshot::Sender<()>>>,
    pub(crate) reader_shutdown_tx: SyncMutex<Option<oneshot::Sender<()>>>,
    pub(crate) clipboard_subscription: SyncMutex<Option<ClipboardSubscription>>,
}

pub(crate) fn desktop_monitors(width: u16, height: u16, dpi: u16) -> RdpResult<Vec<gcc::Monitor>> {
    let dpi = match dpi {
        0 => desktop_dpi()?,
        value => u32::from(value),
    };
    Ok(vec![gcc::Monitor::from_size(width & !1, height & !1, dpi, true)?])
}

fn desktop_dpi() -> RdpResult<u32> {
    #[cfg(windows)]
    {
        windows_desktop_dpi()
    }
    #[cfg(target_os = "macos")]
    {
        macos_desktop_dpi()
    }
    #[cfg(target_os = "linux")]
    {
        linux_desktop_dpi()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Ok(96)
    }
}

#[cfg(windows)]
type WindowsHandle = *mut core::ffi::c_void;

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetDC(hwnd: WindowsHandle) -> WindowsHandle;
    fn ReleaseDC(hwnd: WindowsHandle, hdc: WindowsHandle) -> i32;
}

#[cfg(windows)]
#[link(name = "gdi32")]
extern "system" {
    fn GetDeviceCaps(hdc: WindowsHandle, index: i32) -> i32;
}

#[cfg(windows)]
fn windows_desktop_dpi() -> RdpResult<u32> {
    use crate::module::rdp::protocol::core::error::RdpProtocolError;
    const LOGPIXELSX: i32 = 88;
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        if hdc.is_null() {
            return Err(RdpProtocolError::protocol("GetDC failed".to_string()));
        }
        let dpi = GetDeviceCaps(hdc, LOGPIXELSX);
        if ReleaseDC(std::ptr::null_mut(), hdc) == 0 {
            return Err(RdpProtocolError::protocol("ReleaseDC failed".to_string()));
        }
        if dpi <= 0 {
            return Err(RdpProtocolError::protocol(format!("GetDeviceCaps(LOGPIXELSX) returned {}", dpi)));
        }
        Ok(dpi as u32)
    }
}

#[cfg(target_os = "macos")]
type ObjcId = *mut core::ffi::c_void;
#[cfg(target_os = "macos")]
type ObjcSel = *const core::ffi::c_void;

#[cfg(target_os = "macos")]
#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const core::ffi::c_char) -> ObjcId;
    fn sel_registerName(name: *const core::ffi::c_char) -> ObjcSel;
    fn objc_msgSend();
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
#[link(name = "objc")]
extern "C" {
    fn objc_msgSend_fpret();
}

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[cfg(target_os = "macos")]
fn macos_desktop_dpi() -> RdpResult<u32> {
    use crate::module::rdp::protocol::core::error::RdpProtocolError;
    unsafe {
        let class = objc_getClass(b"NSScreen\0".as_ptr() as *const core::ffi::c_char);
        if class.is_null() {
            return Err(RdpProtocolError::protocol("objc_getClass(NSScreen) failed".to_string()));
        }
        let main_screen_sel = sel_registerName(b"mainScreen\0".as_ptr() as *const core::ffi::c_char);
        let scale_sel = sel_registerName(b"backingScaleFactor\0".as_ptr() as *const core::ffi::c_char);
        if main_screen_sel.is_null() || scale_sel.is_null() {
            return Err(RdpProtocolError::protocol("sel_registerName failed".to_string()));
        }
        let main_screen: ObjcId = std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId>(objc_msgSend)(class, main_screen_sel);
        if main_screen.is_null() {
            return Err(RdpProtocolError::protocol("NSScreen.mainScreen is null".to_string()));
        }
        let scale = macos_backing_scale(main_screen, scale_sel);
        if !(scale > 0.0) || !scale.is_finite() {
            return Err(RdpProtocolError::protocol(format!("invalid backingScaleFactor: {scale}")));
        }
        let dpi = (scale * 96.0).round();
        if dpi < 1.0 || dpi > f64::from(u32::MAX) {
            return Err(RdpProtocolError::protocol(format!("backingScaleFactor to DPI conversion overflow: {scale}")));
        }
        Ok(dpi as u32)
    }
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
unsafe fn macos_backing_scale(screen: ObjcId, scale_sel: ObjcSel) -> f64 {
    std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn(ObjcId, ObjcSel) -> f64>(objc_msgSend_fpret)(screen, scale_sel)
}

#[cfg(all(target_os = "macos", not(target_arch = "x86_64")))]
unsafe fn macos_backing_scale(screen: ObjcId, scale_sel: ObjcSel) -> f64 {
    std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn(ObjcId, ObjcSel) -> f64>(objc_msgSend)(screen, scale_sel)
}

#[cfg(target_os = "linux")]
type X11Ptr = *mut core::ffi::c_void;

#[cfg(target_os = "linux")]
#[link(name = "X11")]
extern "C" {
    fn XOpenDisplay(name: *const core::ffi::c_char) -> X11Ptr;
    fn XCloseDisplay(display: X11Ptr) -> i32;
    fn XDefaultScreenOfDisplay(display: X11Ptr) -> X11Ptr;
    fn XWidthOfScreen(screen: X11Ptr) -> i32;
    fn XWidthMMOfScreen(screen: X11Ptr) -> i32;
}

#[cfg(target_os = "linux")]
fn linux_desktop_dpi() -> RdpResult<u32> {
    use crate::module::rdp::protocol::core::error::RdpProtocolError;
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(RdpProtocolError::protocol("XOpenDisplay failed".to_string()));
        }
        let screen = XDefaultScreenOfDisplay(display);
        if screen.is_null() {
            XCloseDisplay(display);
            return Err(RdpProtocolError::protocol("XDefaultScreenOfDisplay is null".to_string()));
        }
        let width = XWidthOfScreen(screen);
        let mm = XWidthMMOfScreen(screen);
        XCloseDisplay(display);
        if width <= 0 {
            return Err(RdpProtocolError::protocol(format!("XWidthOfScreen returned {width}")));
        }
        if mm <= 0 {
            return Err(RdpProtocolError::protocol(format!("XWidthMMOfScreen returned {mm}")));
        }
        let dpi = f64::from(width) * 25.4 / f64::from(mm);
        if !(dpi > 0.0) || !dpi.is_finite() {
            return Err(RdpProtocolError::protocol(format!("invalid X11 DPI: width={width} mm={mm}")));
        }
        let dpi = dpi.round();
        if dpi < 1.0 || dpi > f64::from(u32::MAX) {
            return Err(RdpProtocolError::protocol(format!("X11 DPI overflow: {dpi}")));
        }
        Ok(dpi as u32)
    }
}
