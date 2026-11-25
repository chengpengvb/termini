#![cfg(target_os = "windows")]

use uiautomation::UIAutomation;
use windows::Win32::Foundation::HWND;
use uiautomation::types::{Handle, TreeScope, UIProperty};
use desktop_core::{DesktopError, DesktopNode, DesktopResult, DesktopSnapshot, DesktopWindow, SnapshotLimit};
use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, GetForegroundWindow, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE};

fn automation() -> DesktopResult<UIAutomation> {
    UIAutomation::new().map_err(|e| DesktopError::Failed(format!("UIAutomation initialization failed: {e}")))
}

fn foreground_hwnd() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}
pub fn list_windows() -> DesktopResult<Vec<DesktopWindow>> {
    let automation = automation()?;
    let root = automation.get_root_element().map_err(|e| DesktopError::Failed(format!("failed to get desktop root element: {e}")))?;
    let condition = automation.create_true_condition().map_err(|e| DesktopError::Failed(format!("failed to create condition: {e}")))?;
    let children = root.find_all(TreeScope::Children, &condition).map_err(|e| DesktopError::Failed(format!("failed to enumerate top-level windows: {e}")))?;
    let focused = foreground_hwnd();
    let mut windows = Vec::new();
    for element in children {
        let title = element.get_name().unwrap_or_default();
        if title.trim().is_empty() {
            continue;
        }
        if element.is_offscreen().unwrap_or(false) {
            continue;
        }
        let Ok(handle) = element.get_native_window_handle() else {
            continue;
        };
        let hwnd: isize = handle.into();
        if hwnd == 0 {
            continue;
        }
        let app = element.get_classname().unwrap_or_default();
        windows.push(DesktopWindow { id: hwnd.to_string(), title, app, focused: hwnd == focused });
    }
    Ok(windows)
}

pub fn snapshot(window_id: Option<&str>, limit: SnapshotLimit) -> DesktopResult<DesktopSnapshot> {
    let hwnd = resolve_hwnd(window_id)?;
    let automation = automation()?;
    let element = automation.element_from_handle(Handle::from(hwnd)).map_err(|e| DesktopError::WindowNotFound(format!("hwnd={hwnd}: {e}")))?;
    let title = element.get_name().unwrap_or_default();
    let app = element.get_classname().unwrap_or_default();
    let window = DesktopWindow { id: hwnd.to_string(), title, app, focused: hwnd == foreground_hwnd() };

    let cache_request = automation.create_cache_request().map_err(|e| DesktopError::Failed(format!("failed to create cache request: {e}")))?;
    let properties = [UIProperty::ControlType, UIProperty::Name, UIProperty::ValueValue, UIProperty::BoundingRectangle, UIProperty::IsEnabled, UIProperty::HasKeyboardFocus, UIProperty::IsOffscreen];
    for property in &properties {
        cache_request.add_property(*property).map_err(|e| DesktopError::Failed(format!("failed to add cached property: {e}")))?;
    }
    cache_request.set_tree_scope(TreeScope::Subtree).map_err(|e| DesktopError::Failed(format!("failed to set cache tree scope: {e}")))?;
    let condition = automation.create_true_condition().map_err(|e| DesktopError::Failed(format!("failed to create condition: {e}")))?;
    let cached_root = element.find_first_build_cache(TreeScope::Element, &condition, &cache_request).map_err(|e| DesktopError::Failed(format!("failed to build cached tree: {e}")))?;

    let mut counter = 0usize;
    let mut truncated = false;
    let root = build_node(&cached_root, 0, &limit, &mut counter, &mut truncated);
    Ok(DesktopSnapshot { window, root, truncated })
}

pub fn focus_window(window_id: &str) -> DesktopResult<()> {
    let hwnd = resolve_hwnd(Some(window_id))?;
    unsafe {
        let hwnd_win = HWND(hwnd as *mut _);
        if IsIconic(hwnd_win).as_bool() {
            let _ = ShowWindow(hwnd_win, SW_RESTORE);
        }
        let _ = BringWindowToTop(hwnd_win);
        let _ = SetForegroundWindow(hwnd_win);
    }
    Ok(())
}

fn resolve_hwnd(window_id: Option<&str>) -> DesktopResult<isize> {
    match window_id {
        Some(raw) => raw.trim().parse::<isize>().map_err(|_| DesktopError::WindowNotFound(format!("invalid window id: {raw}"))),
        None => {
            let hwnd = foreground_hwnd();
            if hwnd == 0 {
                return Err(DesktopError::WindowNotFound("no foreground window".to_string()));
            }
            Ok(hwnd)
        }
    }
}

fn build_node(element: &uiautomation::UIElement, depth: usize, limit: &SnapshotLimit, counter: &mut usize, truncated: &mut bool) -> DesktopNode {
    let eid = *counter;
    *counter += 1;

    let role = element.get_cached_control_type().map(|ct| format!("{ct:?}")).unwrap_or_else(|_| "Unknown".to_string());
    let name = element.get_cached_name().unwrap_or_default();
    let value: Option<String> = element.get_cached_property_value(UIProperty::ValueValue).ok().and_then(|variant| TryInto::<String>::try_into(variant).ok()).filter(|text| !text.is_empty());
    let rect = element.get_cached_bounding_rectangle().ok().map(|r| (r.get_left(), r.get_top(), r.get_width(), r.get_height())).filter(|(_, _, w, h)| *w > 0 && *h > 0);
    let enabled = element.is_cached_enabled().unwrap_or(true);
    let focused = element.has_cached_keyboard_focus().unwrap_or(false);

    let mut node = DesktopNode { eid, role, name, value, rect, enabled, focused, visible: true, pressable: false, children: Vec::new() };

    if depth >= limit.max_depth || *counter >= limit.max_nodes {
        if depth < limit.max_depth {
            *truncated = true;
        }
        return node;
    }
    if let Ok(children) = element.get_cached_children() {
        for child in children {
            if *counter >= limit.max_nodes {
                *truncated = true;
                break;
            }
            if child.is_cached_offscreen().unwrap_or(false) {
                continue;
            }
            node.children.push(build_node(&child, depth + 1, limit, counter, truncated));
        }
    }
    node
}

pub fn activate_window(window_id: Option<&str>) -> DesktopResult<()> {
    let Some(id) = window_id else {
        return Ok(());
    };
    focus_window(id)
}

pub fn press_element(eid: usize) -> DesktopResult<String> {
    Err(DesktopError::Failed(format!("element actions are only available on macOS (eid {eid})")))
}

pub fn focus_element(eid: usize) -> DesktopResult<()> {
    Err(DesktopError::Failed(format!("element focus is only available on macOS (eid {eid})")))
}

pub fn set_element_text(eid: usize, _text: &str) -> DesktopResult<()> {
    Err(DesktopError::Failed(format!("element value write is only available on macOS (eid {eid})")))
}

pub fn element_box(eid: usize) -> DesktopResult<desktop_core::ElementBox> {
    Err(DesktopError::Failed(format!("element box is only available on macOS (eid {eid})")))
}
