#![cfg(target_os = "macos")]

use std::ffi::c_void;
use accessibility_sys;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use std::sync::atomic::{AtomicBool, Ordering};
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFRelease, CFRetain, CFType, CFTypeRef, TCFType};
use desktop_core::{DesktopError, DesktopNode, DesktopResult, DesktopSnapshot, DesktopWindow, SnapshotLimit};
use core_graphics::display::{kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly, CGWindowListCopyWindowInfo};

const ATTR_ROLE: usize = 0;
const ATTR_TITLE: usize = 1;
const ATTR_VALUE: usize = 3;
const ATTR_SIZE: usize = 5;
const ATTR_ENABLED: usize = 6;
const ATTR_FOCUSED: usize = 7;
const ATTR_POSITION: usize = 4;
const ATTR_CHILDREN: usize = 8;
const AX_TIMEOUT_SECS: f32 = 1.0;
const ATTR_DESCRIPTION: usize = 2;
const NORMAL_WINDOW_LAYER: i64 = 0;

const ATTR_HELP: usize = 10;
const PROBE_GRID_COLS: i32 = 4;
const PROBE_GRID_ROWS: i32 = 6;
const PROBE_CLIMB_MAX: usize = 8;
const PROBE_MAX_REGIONS: usize = 3;
const PROBE_MIN_AREA: i64 = 40_000;
const ATTR_PLACEHOLDER: usize = 11;
const ATTR_TITLE_UI: usize = 12;
const MENU_BAR_ROLE: &str = "MenuBar";
const PRUNE_MIN_DEPTH: usize = 2;
const PRESS_QUERY_MAX: usize = 500;
const ACTIVATE_SETTLE_MS: u64 = 300;
const PROBE_BUDGET_MS: u64 = 2_500;
const UNKNOWN_ROLE: &str = "Unknown";
const CHROMIUM_AX_SETTLE_MS: u64 = 150;
const ATTR_ROLE_DESCRIPTION: usize = 9;
const MAX_CHILDREN_PER_NODE: usize = 128;
const LOG_PREFIX: &str = "[desktop]";
static LOGGED_API_DISABLED: AtomicBool = AtomicBool::new(false);
const ATTR_MANUAL_ACCESSIBILITY: &str = "AXManualAccessibility";
const ATTR_ENHANCED_USER_INTERFACE: &str = "AXEnhancedUserInterface";

struct CgWindow {
    pid: i64,
    number: i64,
    app: String,
    title: String,
    bounds: Option<(i32, i32, i32, i32)>,
}

struct Viewport {
    rects: Vec<(i32, i32, i32, i32)>,
}

struct ElementHandle(accessibility_sys::AXUIElementRef);

unsafe impl Send for ElementHandle {}

impl Drop for ElementHandle {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0 as _) };
    }
}

#[derive(Default)]
struct HandleCache {
    window: String,
    items: std::collections::HashMap<usize, ElementHandle>,
}

struct BuildContext<'a> {
    limit: &'a SnapshotLimit,
    names: &'a CFArray<CFString>,
    viewport: &'a Viewport,
    counter: &'a mut usize,
    truncated: &'a mut bool,
    presses: usize,
    pruned: usize,
    handles: std::collections::HashMap<usize, ElementHandle>,
}

impl Viewport {
    fn covers(&self, rect: (i32, i32, i32, i32)) -> bool {
        let (x, y, width, height) = rect;
        let (cx, cy) = (x + width / 2, y + height / 2);
        self.rects.iter().any(|area| point_inside(cx, cy, *area))
    }

    fn touches(&self, rect: (i32, i32, i32, i32)) -> bool {
        self.rects.iter().any(|area| rect_overlaps(rect, *area))
    }

    fn empty(&self) -> bool {
        self.rects.is_empty()
    }
}

fn point_inside(x: i32, y: i32, area: (i32, i32, i32, i32)) -> bool {
    x >= area.0 && y >= area.1 && x <= area.0 + area.2 && y <= area.1 + area.3
}

fn rect_overlaps(left: (i32, i32, i32, i32), right: (i32, i32, i32, i32)) -> bool {
    left.0 < right.0 + right.2 && right.0 < left.0 + left.2 && left.1 < right.1 + right.3 && right.1 < left.1 + left.3
}

fn handle_cache() -> &'static Mutex<HandleCache> {
    static CELL: OnceLock<Mutex<HandleCache>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(HandleCache::default()))
}

pub fn list_windows() -> DesktopResult<Vec<DesktopWindow>> {
    log_process_trust("list_windows");
    let list = list_cg_windows()?;
    let mut windows = Vec::with_capacity(list.len());
    for (index, item) in list.into_iter().enumerate() {
        let display_title = if item.title.is_empty() { item.app.clone() } else { item.title };
        windows.push(DesktopWindow { id: format!("{}:{}", item.pid, item.number), title: display_title, app: item.app, focused: index == 0 });
    }
    desktop_log(&format!("list_windows 可见={}", windows.len()));
    Ok(windows)
}

pub fn snapshot(window_id: Option<&str>, limit: SnapshotLimit) -> DesktopResult<DesktopSnapshot> {
    log_process_trust("snapshot");
    desktop_log(&format!("snapshot 开始 window_id={:?} max_depth={} max_nodes={}", window_id, limit.max_depth, limit.max_nodes));
    let (window_ref, window) = match resolve_window(window_id) {
        Ok(resolved) => resolved,
        Err(error) => {
            desktop_log(&format!("snapshot 解析窗口失败 window_id={:?} err={error}", window_id));
            return Err(error);
        }
    };
    desktop_log(&format!("snapshot 窗口 id={} title={} app={}", window.id, window.title, window.app));
    let names = node_attr_names();
    let viewport = build_viewport(window_ref);
    let mut counter = 0usize;
    let mut truncated = false;
    let mut context =
        BuildContext { limit: &limit, names: &names, viewport: &viewport, counter: &mut counter, truncated: &mut truncated, presses: 0, pruned: 0, handles: std::collections::HashMap::new() };
    let pid = element_pid(window_ref).unwrap_or_default();
    let app = create_app(pid);
    let mut root = build_node(app, 0, &mut context);
    probe_hidden_regions(app, &mut root, &mut context);
    let presses = context.presses;
    let pruned = context.pruned;
    let handles = std::mem::take(&mut context.handles);
    drop(context);
    store_handles(&window.id, handles);
    unsafe {
        CFRelease(app as _);
        CFRelease(window_ref as _);
    }
    let visible = root.interactive_count();
    desktop_log(&format!("snapshot 完成 pid={pid} nodes={counter} 可见可用={visible} 视口={} 裁剪={pruned} 可点查询={presses} truncated={truncated}", viewport.rects.len()));
    Ok(DesktopSnapshot { window, root, truncated })
}

fn build_viewport(window_ref: accessibility_sys::AXUIElementRef) -> Viewport {
    let own = element_rect(window_ref);
    let sheets = collect_child_rects(window_ref, "AXSheets");
    if !sheets.is_empty() {
        desktop_log(&format!("视口 收敛到 AXSheet 数量={}", sheets.len()));
        return Viewport { rects: sheets };
    }
    let Some(pid) = element_pid(window_ref) else {
        return Viewport { rects: own.into_iter().collect() };
    };
    let app = create_app(pid);
    let mut rects = collect_child_rects(app, "AXWindows");
    unsafe { CFRelease(app as _) };
    if let Some(rect) = own {
        let known = rects.iter().any(|item| *item == rect);
        if !known {
            rects.push(rect);
        }
    }
    Viewport { rects }
}

fn collect_child_rects(element: accessibility_sys::AXUIElementRef, attribute: &str) -> Vec<(i32, i32, i32, i32)> {
    let Some(raw) = copy_attribute(element, attribute, false) else {
        return Vec::new();
    };
    let array: CFArray<CFType> = unsafe { CFArray::wrap_under_create_rule(raw as _) };
    array.iter().filter_map(|item| element_rect(item.as_CFTypeRef() as accessibility_sys::AXUIElementRef)).collect()
}

fn store_handles(window: &str, items: std::collections::HashMap<usize, ElementHandle>) {
    let mut cache = handle_cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.window = window.to_string();
    cache.items = items;
}

pub fn focus_window(window_id: &str) -> DesktopResult<()> {
    log_process_trust("focus_window");
    desktop_log(&format!("focus_window 开始 window_id={window_id}"));
    let (window_ref, meta) = match resolve_window(Some(window_id)) {
        Ok(resolved) => resolved,
        Err(error) => {
            desktop_log(&format!("focus_window 解析窗口失败 window_id={window_id} err={error}"));
            return Err(error);
        }
    };
    let pid: i32 = meta.id.split(':').next().and_then(|p| p.parse().ok()).unwrap_or(0);
    unsafe {
        let app = create_app(pid);
        let frontmost = CFString::new("AXFrontmost");
        let set_err = accessibility_sys::AXUIElementSetAttributeValue(app, frontmost.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
        let raise = CFString::new("AXRaise");
        let raise_err = accessibility_sys::AXUIElementPerformAction(window_ref, raise.as_concrete_TypeRef());
        desktop_log(&format!("focus_window pid={pid} title={} AXFrontmost={} AXRaise={}", meta.title, ax_error_name(set_err), ax_error_name(raise_err)));
        CFRelease(app as _);
        CFRelease(window_ref as _);
    }
    Ok(())
}

pub fn activate_window(window_id: Option<&str>) -> DesktopResult<()> {
    let (window_ref, meta) = resolve_window(window_id)?;
    let pid: i32 = meta.id.split(':').next().and_then(|part| part.parse().ok()).unwrap_or(0);
    let already = meta.focused && window_id.is_none();
    unsafe {
        let app = create_app(pid);
        let frontmost = CFString::new("AXFrontmost");
        let set_err = accessibility_sys::AXUIElementSetAttributeValue(app, frontmost.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
        let raise = CFString::new(accessibility_sys::kAXRaiseAction);
        let raise_err = accessibility_sys::AXUIElementPerformAction(window_ref, raise.as_concrete_TypeRef());
        desktop_log(&format!("激活窗口 pid={pid} app={} AXFrontmost={} AXRaise={}", meta.app, ax_error_name(set_err), ax_error_name(raise_err)));
        CFRelease(app as _);
        CFRelease(window_ref as _);
    }
    if already {
        return Ok(());
    }
    std::thread::sleep(std::time::Duration::from_millis(ACTIVATE_SETTLE_MS));
    Ok(())
}

pub fn press_element(eid: usize) -> DesktopResult<String> {
    with_handle(eid, |element| {
        for action in [accessibility_sys::kAXPressAction, accessibility_sys::kAXPickAction, accessibility_sys::kAXConfirmAction] {
            let name = CFString::new(action);
            let err = unsafe { accessibility_sys::AXUIElementPerformAction(element, name.as_concrete_TypeRef()) };
            if err == accessibility_sys::kAXErrorSuccess {
                desktop_log(&format!("元素动作 eid={eid} {action}=Success"));
                return Ok(action.to_string());
            }
            desktop_log(&format!("元素动作 eid={eid} {action}={}", ax_error_name(err)));
        }
        Err(DesktopError::Failed(format!("eid {eid} 不支持 AX 动作")))
    })
}

pub fn focus_element(eid: usize) -> DesktopResult<()> {
    with_handle(eid, |element| {
        let focused = CFString::new("AXFocused");
        let err = unsafe { accessibility_sys::AXUIElementSetAttributeValue(element, focused.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef()) };
        desktop_log(&format!("元素聚焦 eid={eid} AXFocused={}", ax_error_name(err)));
        if err != accessibility_sys::kAXErrorSuccess {
            return Err(DesktopError::Failed(format!("eid {eid} 无法聚焦: {}", ax_error_name(err))));
        }
        Ok(())
    })
}

pub fn set_element_text(eid: usize, text: &str) -> DesktopResult<()> {
    with_handle(eid, |element| {
        let attr = CFString::new("AXValue");
        let mut settable = false;
        let check = unsafe { accessibility_sys::AXUIElementIsAttributeSettable(element, attr.as_concrete_TypeRef(), &mut settable) };
        if check != accessibility_sys::kAXErrorSuccess || !settable {
            desktop_log(&format!("元素写值 eid={eid} AXValue 不可写 check={}", ax_error_name(check)));
            return Err(DesktopError::Failed(format!("eid {eid} 的 AXValue 不可写")));
        }
        let value = CFString::new(text);
        let err = unsafe { accessibility_sys::AXUIElementSetAttributeValue(element, attr.as_concrete_TypeRef(), value.as_CFTypeRef()) };
        desktop_log(&format!("元素写值 eid={eid} len={} AXValue={}", text.chars().count(), ax_error_name(err)));
        if err != accessibility_sys::kAXErrorSuccess {
            return Err(DesktopError::Failed(format!("eid {eid} 写入失败: {}", ax_error_name(err))));
        }
        Ok(())
    })
}

pub fn element_box(eid: usize) -> DesktopResult<desktop_core::ElementBox> {
    with_handle(eid, |element| {
        let Some(rect) = element_rect(element) else {
            return Err(DesktopError::Failed(format!("eid {eid} 没有坐标")));
        };
        let window = element_window_rect(element);
        let visible = window.is_some_and(|area| point_inside(rect.0 + rect.2 / 2, rect.1 + rect.3 / 2, area));
        Ok(desktop_core::ElementBox { rect, window, visible })
    })
}

fn element_window_rect(element: accessibility_sys::AXUIElementRef) -> Option<(i32, i32, i32, i32)> {
    let raw = copy_attribute(element, "AXWindow", false)?;
    let rect = element_rect(raw as accessibility_sys::AXUIElementRef);
    unsafe { CFRelease(raw as _) };
    rect
}

fn with_handle<T>(eid: usize, apply: impl FnOnce(accessibility_sys::AXUIElementRef) -> DesktopResult<T>) -> DesktopResult<T> {
    let cache = handle_cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(handle) = cache.items.get(&eid) else {
        return Err(DesktopError::ElementNotFound(format!("eid {eid} 不在最近一次快照里，先重新读一次 desktop_snapshot")));
    };
    apply(handle.0)
}

fn resolve_window(window_id: Option<&str>) -> DesktopResult<(accessibility_sys::AXUIElementRef, DesktopWindow)> {
    match window_id {
        Some(raw) => resolve_window_by_id(raw),
        None => resolve_focused_window(),
    }
}

fn resolve_window_by_id(raw: &str) -> DesktopResult<(accessibility_sys::AXUIElementRef, DesktopWindow)> {
    let (pid, token) = parse_window_id(raw)?;
    desktop_log(&format!("resolve_window 按id pid={pid} token={token} raw={raw}"));
    let app = create_app(pid);
    ensure_accessibility_awake(app, pid);
    let cg = find_cg_window(pid, token);
    let Some(array) = copy_ax_windows(app) else {
        return take_app_primary_window(app, raw, cg.as_ref());
    };
    desktop_log(&format!("resolve_window 应用 {pid} AXWindows 数量={}", array.len()));
    let bounds = cg.as_ref().and_then(|window| window.bounds);
    let Some(window_ref) = pick_ax_window(&array, bounds) else {
        unsafe { CFRelease(app as _) };
        desktop_log(&format!("resolve_window 未匹配到窗口 raw={raw} len={}", array.len()));
        return Err(DesktopError::WindowNotFound(format!("window not found: {raw}")));
    };
    let id = match &cg {
        Some(window) => format!("{}:{}", window.pid, window.number),
        None => raw.to_string(),
    };
    let app_name = resolve_app_name(app, cg.as_ref());
    finish_window(app, window_ref, &id, false, app_name)
}

fn resolve_focused_window() -> DesktopResult<(accessibility_sys::AXUIElementRef, DesktopWindow)> {
    desktop_log("resolve_window 使用前台窗口");
    if let Some(resolved) = try_ax_system_focused() {
        return Ok(resolved);
    }
    let list = list_cg_windows()?;
    let Some(front) = list.into_iter().next() else {
        return Err(DesktopError::WindowNotFound("no focused application".to_string()));
    };
    desktop_log(&format!("resolve_window AX 系统前台不可用，改用 CG 窗口 pid={} number={}", front.pid, front.number));
    resolve_window_by_id(&format!("{}:{}", front.pid, front.number))
}

fn try_ax_system_focused() -> Option<(accessibility_sys::AXUIElementRef, DesktopWindow)> {
    let system = unsafe { accessibility_sys::AXUIElementCreateSystemWide() };
    let _ = unsafe { accessibility_sys::AXUIElementSetMessagingTimeout(system, AX_TIMEOUT_SECS) };
    let Some(app_ref) = copy_attribute(system, "AXFocusedApplication", true) else {
        unsafe { CFRelease(system as _) };
        desktop_log("AXFocusedApplication 为空，改用 CG 前台窗口");
        return None;
    };
    let app_ref = app_ref as accessibility_sys::AXUIElementRef;
    let app_pid = element_pid(app_ref).unwrap_or(0);
    ensure_accessibility_awake(app_ref, app_pid);
    let window_raw = copy_attribute(app_ref, "AXFocusedWindow", true).or_else(|| copy_attribute(app_ref, "AXMainWindow", false));
    let Some(window_raw) = window_raw else {
        unsafe {
            CFRelease(app_ref as _);
            CFRelease(system as _);
        }
        desktop_log("前台应用没有 AXFocusedWindow/AXMainWindow，改用 CG 前台窗口");
        return None;
    };
    let window_ref = window_raw as accessibility_sys::AXUIElementRef;
    let pid = element_pid(window_ref).unwrap_or(app_pid);
    let title = element_string(window_ref, "AXTitle").unwrap_or_default();
    let cg = find_cg_by_pid_bounds(pid, element_rect(window_ref));
    let id = match &cg {
        Some(window) => format!("{}:{}", window.pid, window.number),
        None => format!("{pid}:0"),
    };
    let app_name = resolve_app_name(app_ref, cg.as_ref());
    unsafe {
        CFRelease(app_ref as _);
        CFRelease(system as _);
    }
    desktop_log(&format!("resolve_window AX 前台窗口 id={id} title={title} app={app_name}"));
    Some((window_ref, DesktopWindow { id, title, app: app_name, focused: true }))
}

fn take_app_primary_window(app: accessibility_sys::AXUIElementRef, raw: &str, cg: Option<&CgWindow>) -> DesktopResult<(accessibility_sys::AXUIElementRef, DesktopWindow)> {
    let window_raw = copy_attribute(app, "AXFocusedWindow", false).or_else(|| copy_attribute(app, "AXMainWindow", false));
    let Some(window_raw) = window_raw else {
        unsafe { CFRelease(app as _) };
        desktop_log(&format!("resolve_window 没有 AXWindows/AXFocusedWindow/AXMainWindow raw={raw}"));
        return Err(DesktopError::WindowNotFound(format!("application has no accessible windows: {raw}")));
    };
    let app_name = resolve_app_name(app, cg);
    finish_window(app, window_raw as accessibility_sys::AXUIElementRef, raw, true, app_name)
}

fn finish_window(
    app: accessibility_sys::AXUIElementRef,
    window_ref: accessibility_sys::AXUIElementRef,
    raw: &str,
    focused: bool,
    app_name: String,
) -> DesktopResult<(accessibility_sys::AXUIElementRef, DesktopWindow)> {
    let title = element_string(window_ref, "AXTitle").unwrap_or_default();
    unsafe { CFRelease(app as _) };
    Ok((window_ref, DesktopWindow { id: raw.to_string(), title, app: app_name, focused }))
}

fn parse_window_id(raw: &str) -> DesktopResult<(i32, i64)> {
    let mut parts = raw.trim().split(':');
    let Some(pid_raw) = parts.next() else {
        return Err(DesktopError::WindowNotFound(format!("invalid window id: {raw}")));
    };
    let pid: i32 = match pid_raw.parse() {
        Ok(value) => value,
        Err(_) => return Err(DesktopError::WindowNotFound(format!("invalid window id: {raw}"))),
    };
    let token: i64 = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
    Ok((pid, token))
}

fn list_cg_windows() -> DesktopResult<Vec<CgWindow>> {
    let raw = unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID) };
    if raw.is_null() {
        desktop_log("CGWindowListCopyWindowInfo 返回空");
        return Err(DesktopError::Failed("CGWindowListCopyWindowInfo returned null".to_string()));
    }
    let list: CFArray<CFDictionary<CFString, CFType>> = unsafe { CFArray::wrap_under_create_rule(raw as _) };
    let mut windows = Vec::new();
    for item in list.iter() {
        let layer = dict_i64(&item, "kCGWindowLayer").unwrap_or(-1);
        if layer != NORMAL_WINDOW_LAYER {
            continue;
        }
        let Some(pid) = dict_i64(&item, "kCGWindowOwnerPID") else {
            continue;
        };
        let Some(number) = dict_i64(&item, "kCGWindowNumber") else {
            continue;
        };
        windows.push(CgWindow {
            pid,
            number,
            app: dict_string(&item, "kCGWindowOwnerName").unwrap_or_default(),
            title: dict_string(&item, "kCGWindowName").unwrap_or_default(),
            bounds: dict_rect(&item, "kCGWindowBounds"),
        });
    }
    Ok(windows)
}

fn find_cg_window(pid: i32, number: i64) -> Option<CgWindow> {
    let list = list_cg_windows().ok()?;
    list.into_iter().find(|window| window.pid == i64::from(pid) && window.number == number)
}

fn find_cg_by_pid_bounds(pid: i32, rect: Option<(i32, i32, i32, i32)>) -> Option<CgWindow> {
    let list = list_cg_windows().ok()?;
    let mut best: Option<(CgWindow, i64)> = None;
    for window in list {
        if window.pid != i64::from(pid) {
            continue;
        }
        let dist = match (rect, window.bounds) {
            (Some(left), Some(right)) => rect_delta(left, right),
            _ => i64::MAX,
        };
        let worse = best.as_ref().is_some_and(|(_, best_dist)| dist >= *best_dist);
        if worse {
            continue;
        }
        best = Some((window, dist));
    }
    best.map(|(window, _)| window)
}

fn copy_ax_windows(app: accessibility_sys::AXUIElementRef) -> Option<CFArray<CFType>> {
    let windows_attr = copy_attribute(app, "AXWindows", true)?;
    Some(unsafe { CFArray::wrap_under_create_rule(windows_attr as _) })
}

fn pick_ax_window(array: &CFArray<CFType>, bounds: Option<(i32, i32, i32, i32)>) -> Option<accessibility_sys::AXUIElementRef> {
    let closest = bounds.and_then(|rect| pick_closest(array, rect));
    if closest.is_some() {
        return closest;
    }
    pick_largest(array)
}

fn pick_closest(array: &CFArray<CFType>, bounds: (i32, i32, i32, i32)) -> Option<accessibility_sys::AXUIElementRef> {
    let mut best: Option<(accessibility_sys::AXUIElementRef, i64)> = None;
    for item in array.iter() {
        let window_ref = item.as_CFTypeRef() as accessibility_sys::AXUIElementRef;
        let Some(rect) = element_rect(window_ref) else {
            continue;
        };
        let dist = rect_delta(rect, bounds);
        let worse = best.as_ref().is_some_and(|(_, best_dist)| dist >= *best_dist);
        if worse {
            continue;
        }
        best = Some((window_ref, dist));
    }
    best.and_then(|(window_ref, _)| retain_ax(window_ref))
}

fn pick_largest(array: &CFArray<CFType>) -> Option<accessibility_sys::AXUIElementRef> {
    let mut best: Option<(accessibility_sys::AXUIElementRef, i64)> = None;
    for item in array.iter() {
        let window_ref = item.as_CFTypeRef() as accessibility_sys::AXUIElementRef;
        let area = element_rect(window_ref).map(|(_, _, width, height)| i64::from(width) * i64::from(height)).unwrap_or(0);
        let worse = best.as_ref().is_some_and(|(_, best_area)| area <= *best_area);
        if worse {
            continue;
        }
        best = Some((window_ref, area));
    }
    best.and_then(|(window_ref, _)| retain_ax(window_ref))
}

fn retain_ax(window_ref: accessibility_sys::AXUIElementRef) -> Option<accessibility_sys::AXUIElementRef> {
    if window_ref.is_null() {
        return None;
    }
    unsafe { CFRetain(window_ref as _) };
    Some(window_ref)
}

fn rect_delta(left: (i32, i32, i32, i32), right: (i32, i32, i32, i32)) -> i64 {
    let (lx, ly, lw, lh) = left;
    let (rx, ry, rw, rh) = right;
    i64::from((lx - rx).abs()) + i64::from((ly - ry).abs()) + i64::from((lw - rw).abs()) + i64::from((lh - rh).abs())
}

fn create_app(pid: i32) -> accessibility_sys::AXUIElementRef {
    let app = unsafe { accessibility_sys::AXUIElementCreateApplication(pid) };
    let _ = unsafe { accessibility_sys::AXUIElementSetMessagingTimeout(app, AX_TIMEOUT_SECS) };
    app
}

fn awakened_pids() -> &'static Mutex<HashSet<i32>> {
    static CELL: OnceLock<Mutex<HashSet<i32>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(HashSet::new()))
}

fn ensure_accessibility_awake(app: accessibility_sys::AXUIElementRef, pid: i32) {
    let mut seen = awakened_pids().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let already_awake = seen.contains(&pid);
    if already_awake {
        return;
    }
    seen.insert(pid);
    drop(seen);
    let manual = CFString::new(ATTR_MANUAL_ACCESSIBILITY);
    let manual_err = unsafe { accessibility_sys::AXUIElementSetAttributeValue(app, manual.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef()) };
    if manual_err == accessibility_sys::kAXErrorSuccess {
        desktop_log(&format!("辅助功能唤醒 pid={pid} AXManualAccessibility=Success"));
        std::thread::sleep(std::time::Duration::from_millis(CHROMIUM_AX_SETTLE_MS));
        return;
    }
    let enhanced = CFString::new(ATTR_ENHANCED_USER_INTERFACE);
    let enhanced_err = unsafe { accessibility_sys::AXUIElementSetAttributeValue(app, enhanced.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef()) };
    desktop_log(&format!("辅助功能唤醒 pid={pid} AXManualAccessibility={} AXEnhancedUserInterface={}", ax_error_name(manual_err), ax_error_name(enhanced_err)));
    if enhanced_err != accessibility_sys::kAXErrorSuccess {
        return;
    }
    std::thread::sleep(std::time::Duration::from_millis(CHROMIUM_AX_SETTLE_MS));
}

fn resolve_app_name(app: accessibility_sys::AXUIElementRef, cg: Option<&CgWindow>) -> String {
    let from_cg = cg.map(|window| window.app.clone()).filter(|name| !name.is_empty());
    if let Some(name) = from_cg {
        return name;
    }
    element_string(app, "AXTitle").unwrap_or_default()
}

fn element_pid(element: accessibility_sys::AXUIElementRef) -> Option<i32> {
    let mut pid = 0;
    let err = unsafe { accessibility_sys::AXUIElementGetPid(element, &mut pid) };
    if err != accessibility_sys::kAXErrorSuccess {
        return None;
    }
    Some(pid)
}

fn node_attr_names() -> CFArray<CFString> {
    CFArray::from_CFTypes(&[
        CFString::new("AXRole"),
        CFString::new("AXTitle"),
        CFString::new("AXDescription"),
        CFString::new("AXValue"),
        CFString::new("AXPosition"),
        CFString::new("AXSize"),
        CFString::new("AXEnabled"),
        CFString::new("AXFocused"),
        CFString::new("AXChildren"),
        CFString::new("AXRoleDescription"),
        CFString::new("AXHelp"),
        CFString::new("AXPlaceholderValue"),
        CFString::new("AXTitleUIElement"),
    ])
}

fn build_node(element: accessibility_sys::AXUIElementRef, depth: usize, context: &mut BuildContext) -> DesktopNode {
    let eid = *context.counter;
    *context.counter += 1;
    let values = copy_many(element, context.names);
    let role = node_role(&values, element);
    let name = node_name(&values, element);
    let value = node_value(&values, element);
    let rect = node_rect(&values, element);
    let enabled = node_bool(&values, element, ATTR_ENABLED, "AXEnabled").unwrap_or(true);
    let focused = node_bool(&values, element, ATTR_FOCUSED, "AXFocused").unwrap_or(false);
    let visible = node_visible(rect, context.viewport);
    let pressable = node_pressable(element, rect, visible, context);
    register_handle(element, eid, context);
    let mut node = DesktopNode { eid, role, name, value, rect, enabled, focused, visible, pressable, children: Vec::new() };
    if node.role == MENU_BAR_ROLE {
        return node;
    }
    if depth >= context.limit.max_depth {
        return node;
    }
    if *context.counter >= context.limit.max_nodes {
        *context.truncated = true;
        return node;
    }
    if prune_offscreen(rect, depth, context) {
        context.pruned += 1;
        return node;
    }
    let Some(array) = node_children(&values, element) else {
        return node;
    };
    for (index, item) in array.iter().enumerate() {
        if index >= MAX_CHILDREN_PER_NODE || *context.counter >= context.limit.max_nodes {
            *context.truncated = true;
            break;
        }
        let child_ref = item.as_CFTypeRef() as accessibility_sys::AXUIElementRef;
        node.children.push(build_node(child_ref, depth + 1, context));
    }
    node
}

fn node_visible(rect: Option<(i32, i32, i32, i32)>, viewport: &Viewport) -> bool {
    if viewport.empty() {
        return true;
    }
    rect.is_some_and(|area| viewport.covers(area))
}

fn prune_offscreen(rect: Option<(i32, i32, i32, i32)>, depth: usize, context: &BuildContext) -> bool {
    if depth < PRUNE_MIN_DEPTH {
        return false;
    }
    if context.viewport.empty() {
        return false;
    }
    let Some(area) = rect else {
        return false;
    };
    !context.viewport.touches(area)
}

fn node_pressable(element: accessibility_sys::AXUIElementRef, rect: Option<(i32, i32, i32, i32)>, visible: bool, context: &mut BuildContext) -> bool {
    if !visible {
        return false;
    }
    if rect.is_none() {
        return false;
    }
    if context.presses >= PRESS_QUERY_MAX {
        return false;
    }
    context.presses += 1;
    element_supports_press(element)
}

fn element_supports_press(element: accessibility_sys::AXUIElementRef) -> bool {
    let mut names: CFArrayRef = std::ptr::null();
    let err = unsafe { accessibility_sys::AXUIElementCopyActionNames(element, &mut names) };
    if err != accessibility_sys::kAXErrorSuccess || names.is_null() {
        return false;
    }
    let array: CFArray<CFString> = unsafe { CFArray::wrap_under_create_rule(names) };
    array.iter().any(|item| item.to_string() == accessibility_sys::kAXPressAction)
}

fn register_handle(element: accessibility_sys::AXUIElementRef, eid: usize, context: &mut BuildContext) {
    let Some(retained) = retain_ax(element) else {
        return;
    };
    context.handles.insert(eid, ElementHandle(retained));
}

struct ProbeContext {
    app: accessibility_sys::AXUIElementRef,
    deadline: std::time::Instant,
    known: HashSet<(i32, i32, i32, i32)>,
    added: usize,
    regions: usize,
}

fn probe_hidden_regions(window_ref: accessibility_sys::AXUIElementRef, root: &mut DesktopNode, build: &mut BuildContext) {
    if !build.limit.probe_hidden {
        return;
    }
    let Some(pid) = element_pid(window_ref) else {
        return;
    };
    let mut rects = Vec::new();
    root.collect_rects(&mut rects);
    let mut probe = ProbeContext {
        app: create_app(pid),
        deadline: std::time::Instant::now() + std::time::Duration::from_millis(PROBE_BUDGET_MS),
        known: rects.into_iter().map(|(_, rect)| rect).collect(),
        added: 0,
        regions: 0,
    };
    let started = std::time::Instant::now();
    probe_node(root, 0, &mut probe, build);
    unsafe { CFRelease(probe.app as _) };
    desktop_log(&format!("hit 探测 pid={pid} 区域={} 新增子树={} nodes={} 耗时={}ms", probe.regions, probe.added, *build.counter, started.elapsed().as_millis()));
}

fn probe_node(node: &mut DesktopNode, depth: usize, probe: &mut ProbeContext, build: &mut BuildContext) {
    let has_children = !node.children.is_empty();
    if has_children {
        for child in node.children.iter_mut() {
            probe_node(child, depth + 1, probe, build);
        }
        return;
    }
    let Some(rect) = node.rect else {
        return;
    };
    if !build.viewport.touches(rect) {
        return;
    }
    if rect_area(rect) < PROBE_MIN_AREA {
        return;
    }
    if probe.regions >= PROBE_MAX_REGIONS {
        *build.truncated = true;
        return;
    }
    probe.regions += 1;
    probe_region(node, depth, rect, probe, build);
}

fn probe_region(node: &mut DesktopNode, depth: usize, bounds: (i32, i32, i32, i32), probe: &mut ProbeContext, build: &mut BuildContext) {
    let (x, y, width, height) = bounds;
    let step_x = (width / (PROBE_GRID_COLS + 1)).max(1);
    let step_y = (height / (PROBE_GRID_ROWS + 1)).max(1);
    let mut row = 1;
    while row <= PROBE_GRID_ROWS {
        let mut col = 1;
        while col <= PROBE_GRID_COLS {
            let exhausted = std::time::Instant::now() >= probe.deadline || *build.counter >= build.limit.max_nodes;
            if exhausted {
                *build.truncated = true;
                return;
            }
            attach_hit(node, depth, bounds, x + step_x * col, y + step_y * row, probe, build);
            col += 1;
        }
        row += 1;
    }
}

fn attach_hit(node: &mut DesktopNode, depth: usize, bounds: (i32, i32, i32, i32), x: i32, y: i32, probe: &mut ProbeContext, build: &mut BuildContext) {
    let Some(hit) = hit_test(probe.app, x, y) else {
        return;
    };
    let target = outermost_inside(hit, bounds);
    let fresh = element_rect(target).filter(|rect| rect_inside(*rect, bounds) && !probe.known.contains(rect));
    let Some(rect) = fresh else {
        unsafe { CFRelease(target as _) };
        return;
    };
    probe.known.insert(rect);
    let child = build_node(target, depth + 1, build);
    unsafe { CFRelease(target as _) };
    probe.added += 1;
    node.children.push(child);
}

fn hit_test(app: accessibility_sys::AXUIElementRef, x: i32, y: i32) -> Option<accessibility_sys::AXUIElementRef> {
    let mut hit: accessibility_sys::AXUIElementRef = std::ptr::null_mut();
    let err = unsafe { accessibility_sys::AXUIElementCopyElementAtPosition(app, x as f32, y as f32, &mut hit) };
    if err != accessibility_sys::kAXErrorSuccess || hit.is_null() {
        return None;
    }
    Some(hit)
}

fn outermost_inside(hit: accessibility_sys::AXUIElementRef, bounds: (i32, i32, i32, i32)) -> accessibility_sys::AXUIElementRef {
    let mut chain: Vec<accessibility_sys::AXUIElementRef> = Vec::new();
    let mut current = hit;
    while chain.len() < PROBE_CLIMB_MAX {
        let Some(raw) = copy_attribute(current, "AXParent", false) else {
            break;
        };
        let parent = raw as accessibility_sys::AXUIElementRef;
        let nested = element_rect(parent).is_some_and(|rect| rect != bounds && rect_inside(rect, bounds));
        if !nested {
            unsafe { CFRelease(parent as _) };
            break;
        }
        chain.push(parent);
        current = parent;
    }
    let Some(top) = chain.pop() else {
        return hit;
    };
    for element in chain {
        unsafe { CFRelease(element as _) };
    }
    unsafe { CFRelease(hit as _) };
    top
}

fn rect_area(rect: (i32, i32, i32, i32)) -> i64 {
    i64::from(rect.2) * i64::from(rect.3)
}

fn rect_inside(inner: (i32, i32, i32, i32), outer: (i32, i32, i32, i32)) -> bool {
    inner.0 >= outer.0 && inner.1 >= outer.1 && inner.0 + inner.2 <= outer.0 + outer.2 && inner.1 + inner.3 <= outer.1 + outer.3
}

fn node_role(values: &Option<CFArray<CFType>>, element: accessibility_sys::AXUIElementRef) -> String {
    let raw = match values {
        Some(items) => attr_string(items, ATTR_ROLE),
        None => element_string(element, "AXRole"),
    };
    let role = raw.unwrap_or_default().trim_start_matches("AX").to_string();
    let precise = !role.is_empty() && role != UNKNOWN_ROLE;
    if precise {
        return role;
    }
    let described = match values {
        Some(items) => attr_string(items, ATTR_ROLE_DESCRIPTION),
        None => element_string(element, "AXRoleDescription"),
    };
    described.filter(|text| !text.is_empty()).unwrap_or_else(|| UNKNOWN_ROLE.to_string())
}

fn node_name(values: &Option<CFArray<CFType>>, element: accessibility_sys::AXUIElementRef) -> String {
    let Some(items) = values else {
        let direct = element_string(element, "AXTitle").or_else(|| element_string(element, "AXDescription")).or_else(|| element_string(element, "AXHelp"));
        return direct.or_else(|| element_string(element, "AXPlaceholderValue")).or_else(|| label_from_title_element(element)).unwrap_or_default();
    };
    let batched = attr_string(items, ATTR_TITLE).or_else(|| attr_string(items, ATTR_DESCRIPTION)).or_else(|| attr_string(items, ATTR_HELP)).or_else(|| attr_string(items, ATTR_PLACEHOLDER));
    let named = batched.filter(|text| !text.is_empty());
    if let Some(text) = named {
        return text;
    }
    attr_label_element(items).or_else(|| label_from_title_element(element)).unwrap_or_default()
}

fn attr_label_element(values: &CFArray<CFType>) -> Option<String> {
    let item = values.get(ATTR_TITLE_UI as isize)?;
    let is_element = unsafe { core_foundation::base::CFGetTypeID(item.as_CFTypeRef()) == accessibility_sys::AXUIElementGetTypeID() };
    if !is_element {
        return None;
    }
    label_text(item.as_CFTypeRef() as accessibility_sys::AXUIElementRef)
}

fn label_from_title_element(element: accessibility_sys::AXUIElementRef) -> Option<String> {
    let raw = copy_attribute(element, "AXTitleUIElement", false)?;
    let label = label_text(raw as accessibility_sys::AXUIElementRef);
    unsafe { CFRelease(raw as _) };
    label
}

fn label_text(element: accessibility_sys::AXUIElementRef) -> Option<String> {
    if element.is_null() {
        return None;
    }
    let text = element_string(element, "AXValue").or_else(|| element_string(element, "AXTitle"));
    text.filter(|value| !value.is_empty())
}

fn node_value(values: &Option<CFArray<CFType>>, element: accessibility_sys::AXUIElementRef) -> Option<String> {
    let raw = match values {
        Some(items) => attr_string(items, ATTR_VALUE),
        None => element_string(element, "AXValue"),
    };
    raw.filter(|text| !text.is_empty())
}

fn node_rect(values: &Option<CFArray<CFType>>, element: accessibility_sys::AXUIElementRef) -> Option<(i32, i32, i32, i32)> {
    match values {
        Some(items) => attr_rect(items).or_else(|| element_rect(element)),
        None => element_rect(element),
    }
}

fn node_bool(values: &Option<CFArray<CFType>>, element: accessibility_sys::AXUIElementRef, index: usize, attribute: &str) -> Option<bool> {
    match values {
        Some(items) => attr_bool(items, index).or_else(|| element_bool(element, attribute)),
        None => element_bool(element, attribute),
    }
}

fn node_children(values: &Option<CFArray<CFType>>, element: accessibility_sys::AXUIElementRef) -> Option<CFArray<CFType>> {
    let batched = values.as_ref().and_then(|items| attr_array(items, ATTR_CHILDREN));
    if batched.is_some() {
        return batched;
    }
    let children_ref = copy_attribute(element, "AXChildren", false)?;
    Some(unsafe { CFArray::wrap_under_create_rule(children_ref as _) })
}

fn copy_many(element: accessibility_sys::AXUIElementRef, names: &CFArray<CFString>) -> Option<CFArray<CFType>> {
    let mut values: CFArrayRef = std::ptr::null();
    let err = unsafe { accessibility_sys::AXUIElementCopyMultipleAttributeValues(element, names.as_concrete_TypeRef(), 0, &mut values) };
    if err != accessibility_sys::kAXErrorSuccess {
        return None;
    }
    if values.is_null() {
        return None;
    }
    Some(unsafe { CFArray::wrap_under_create_rule(values) })
}

fn attr_string(values: &CFArray<CFType>, index: usize) -> Option<String> {
    let item = values.get(index as isize)?;
    if !item.instance_of::<CFString>() {
        return None;
    }
    let value = unsafe { CFString::wrap_under_get_rule(item.as_CFTypeRef() as _) };
    Some(value.to_string())
}

fn attr_bool(values: &CFArray<CFType>, index: usize) -> Option<bool> {
    let item = values.get(index as isize)?;
    if !item.instance_of::<CFBoolean>() {
        return None;
    }
    let value = unsafe { CFBoolean::wrap_under_get_rule(item.as_CFTypeRef() as _) };
    Some(value.into())
}

fn attr_rect(values: &CFArray<CFType>) -> Option<(i32, i32, i32, i32)> {
    let pos = values.get(ATTR_POSITION as isize)?;
    let size = values.get(ATTR_SIZE as isize)?;
    rect_from_ax_values(pos.as_CFTypeRef(), size.as_CFTypeRef())
}

fn attr_array(values: &CFArray<CFType>, index: usize) -> Option<CFArray<CFType>> {
    let item = values.get(index as isize)?;
    if !item.instance_of::<CFArray<*const c_void>>() {
        return None;
    }
    Some(unsafe { CFArray::wrap_under_get_rule(item.as_CFTypeRef() as _) })
}

fn copy_attribute(element: accessibility_sys::AXUIElementRef, attribute: &str, log_failure: bool) -> Option<*const c_void> {
    let attr = CFString::new(attribute);
    let mut value: *const c_void = std::ptr::null();
    let err = unsafe { accessibility_sys::AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value as *mut _ as *mut _) };
    if err != accessibility_sys::kAXErrorSuccess {
        note_ax_error(attribute, err, log_failure);
        return None;
    }
    if value.is_null() {
        if log_failure {
            desktop_log(&format!("AX 属性 {attribute} 成功但值为空"));
        }
        return None;
    }
    Some(value)
}

fn element_string(element: accessibility_sys::AXUIElementRef, attribute: &str) -> Option<String> {
    let raw = copy_attribute(element, attribute, false)?;
    let value: CFType = unsafe { CFType::wrap_under_create_rule(raw as _) };
    value.downcast::<CFString>().map(|s| s.to_string())
}

fn element_bool(element: accessibility_sys::AXUIElementRef, attribute: &str) -> Option<bool> {
    let raw = copy_attribute(element, attribute, false)?;
    let value: CFType = unsafe { CFType::wrap_under_create_rule(raw as _) };
    value.downcast::<CFBoolean>().map(|b| b.into())
}

fn element_rect(element: accessibility_sys::AXUIElementRef) -> Option<(i32, i32, i32, i32)> {
    let pos_raw = copy_attribute(element, "AXPosition", false)?;
    let size_raw = copy_attribute(element, "AXSize", false)?;
    let rect = rect_from_ax_values(pos_raw as CFTypeRef, size_raw as CFTypeRef);
    unsafe {
        CFRelease(pos_raw as _);
        CFRelease(size_raw as _);
    }
    rect
}

fn rect_from_ax_values(pos_raw: CFTypeRef, size_raw: CFTypeRef) -> Option<(i32, i32, i32, i32)> {
    if pos_raw.is_null() || size_raw.is_null() {
        return None;
    }
    let mut point = core_graphics::geometry::CGPoint::new(0.0, 0.0);
    let mut size = core_graphics::geometry::CGSize::new(0.0, 0.0);
    let ok_point = unsafe { accessibility_sys::AXValueGetValue(pos_raw as accessibility_sys::AXValueRef, accessibility_sys::kAXValueTypeCGPoint, &mut point as *mut _ as *mut c_void) };
    let ok_size = unsafe { accessibility_sys::AXValueGetValue(size_raw as accessibility_sys::AXValueRef, accessibility_sys::kAXValueTypeCGSize, &mut size as *mut _ as *mut c_void) };
    if !ok_point || !ok_size || size.width <= 0.0 || size.height <= 0.0 {
        return None;
    }
    Some((point.x as i32, point.y as i32, size.width as i32, size.height as i32))
}

fn dict_i64(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<i64> {
    dict.find(CFString::new(key)).and_then(|value| value.downcast::<CFNumber>()).and_then(|number| number.to_i64())
}

fn dict_f64(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<f64> {
    dict.find(CFString::new(key)).and_then(|value| value.downcast::<CFNumber>()).and_then(|number| number.to_f64())
}

fn dict_string(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<String> {
    dict.find(CFString::new(key)).and_then(|value| value.downcast::<CFString>()).map(|s| s.to_string())
}

fn dict_rect(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<(i32, i32, i32, i32)> {
    let value = dict.find(CFString::new(key))?;
    if !value.instance_of::<CFDictionary>() {
        return None;
    }
    let bounds: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(value.as_CFTypeRef() as _) };
    let x = dict_f64(&bounds, "X")?;
    let y = dict_f64(&bounds, "Y")?;
    let width = dict_f64(&bounds, "Width")?;
    let height = dict_f64(&bounds, "Height")?;
    Some((x as i32, y as i32, width as i32, height as i32))
}

fn log_process_trust(op: &str) {
    let trusted = unsafe { accessibility_sys::AXIsProcessTrusted() };
    let api_enabled = unsafe { accessibility_sys::AXAPIEnabled() };
    desktop_log(&format!("{op} pid={} exe={} accessibility_sys::AXIsProcessTrusted={} accessibility_sys::AXAPIEnabled={}", std::process::id(), process_exe(), trusted, api_enabled));
}

fn process_exe() -> String {
    match std::env::current_exe() {
        Ok(path) => path.display().to_string(),
        Err(error) => format!("<unavailable:{error}>"),
    }
}

fn note_ax_error(attribute: &str, err: i32, log_failure: bool) {
    if err == accessibility_sys::kAXErrorAPIDisabled {
        log_api_disabled_once(attribute);
        return;
    }
    if !log_failure {
        return;
    }
    desktop_log(&format!("AX 读取 {attribute} 失败 err={}", ax_error_name(err)));
}

fn log_api_disabled_once(attribute: &str) {
    if LOGGED_API_DISABLED.swap(true, Ordering::Relaxed) {
        return;
    }
    desktop_log(&format!("辅助功能 API 被禁用(TCC)，当前进程未授权。first_attr={attribute}"));
}

fn ax_error_name(err: i32) -> String {
    if err == accessibility_sys::kAXErrorSuccess {
        return "Success".to_string();
    }
    if err == accessibility_sys::kAXErrorFailure {
        return "Failure".to_string();
    }
    if err == accessibility_sys::kAXErrorIllegalArgument {
        return "IllegalArgument".to_string();
    }
    if err == accessibility_sys::kAXErrorInvalidUIElement {
        return "InvalidUIElement".to_string();
    }
    if err == accessibility_sys::kAXErrorCannotComplete {
        return "CannotComplete".to_string();
    }
    if err == accessibility_sys::kAXErrorAttributeUnsupported {
        return "AttributeUnsupported".to_string();
    }
    if err == accessibility_sys::kAXErrorActionUnsupported {
        return "ActionUnsupported".to_string();
    }
    if err == accessibility_sys::kAXErrorNotImplemented {
        return "NotImplemented".to_string();
    }
    if err == accessibility_sys::kAXErrorAPIDisabled {
        return "APIDisabled".to_string();
    }
    if err == accessibility_sys::kAXErrorNoValue {
        return "NoValue".to_string();
    }
    format!("{err}")
}

fn desktop_log(message: &str) {
    eprintln!("{LOG_PREFIX} {message}");
}
