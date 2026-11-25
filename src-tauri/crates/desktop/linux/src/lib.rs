#![cfg(target_os = "linux")]

use std::pin::Pin;
use zbus::Connection;
use std::future::Future;
use zbus::names::UniqueName;
use zbus::zvariant::ObjectPath;
use std::sync::{mpsc, OnceLock};
use atspi::proxy::proxy_ext::ProxyExt;
use tokio::runtime::{Builder, Runtime};
use atspi::proxy::accessible::{AccessibleProxy, ObjectRefExt};
use atspi::{AccessibilityConnection, CoordType, ObjectRef, ObjectRefOwned, State};
use desktop_core::{DesktopError, DesktopNode, DesktopResult, DesktopSnapshot, DesktopWindow, SnapshotLimit};

const VALUE_TEXT_MAX_CHARS: i32 = 200;

fn atspi_jobs() -> &'static mpsc::Sender<Box<dyn FnOnce(&Runtime) + Send>> {
    static TX: OnceLock<mpsc::Sender<Box<dyn FnOnce(&Runtime) + Send>>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Box<dyn FnOnce(&Runtime) + Send>>();
        std::thread::Builder::new()
            .name("atspi".into())
            .spawn(move || {
                let rt = Builder::new_current_thread().enable_all().build().expect("atspi runtime");
                while let Ok(job) = rx.recv() {
                    job(&rt);
                }
            })
            .expect("spawn atspi thread");
        tx
    })
}

fn run_atspi<T, F, Fut>(make: F) -> T
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = T> + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::sync_channel(1);
    atspi_jobs()
        .send(Box::new(move |rt| {
            let _ = tx.send(rt.block_on(make()));
        }))
        .expect("atspi thread alive");
    rx.recv().expect("atspi result")
}

pub fn list_windows() -> DesktopResult<Vec<DesktopWindow>> {
    run_atspi(list_windows_async)
}

pub fn snapshot(window_id: Option<&str>, limit: SnapshotLimit) -> DesktopResult<DesktopSnapshot> {
    let window_id = window_id.map(str::to_string);
    run_atspi(move || snapshot_async(window_id, limit))
}

pub fn focus_window(window_id: &str) -> DesktopResult<()> {
    let window_id = window_id.to_string();
    run_atspi(move || focus_async(window_id))
}

async fn connect() -> DesktopResult<AccessibilityConnection> {
    AccessibilityConnection::new().await.map_err(|e| DesktopError::Failed(format!("failed to connect to AT-SPI bus (desktop accessibility not enabled?): {e}")))
}

async fn list_windows_async() -> DesktopResult<Vec<DesktopWindow>> {
    let conn = connect().await?;
    let bus = conn.connection();
    let root = conn.root_accessible_on_registry().await.map_err(|e| DesktopError::Failed(format!("failed to read AT-SPI root: {e}")))?;
    let apps = root.get_children().await.unwrap_or_default();
    let mut windows = Vec::new();
    for app_ref in &apps {
        let Ok(app) = app_ref.as_accessible_proxy(bus).await else {
            continue;
        };
        let app_name = app.name().await.unwrap_or_default();
        let children = app.get_children().await.unwrap_or_default();
        if children.is_empty() {
            windows.push(DesktopWindow { id: object_ref_id(app_ref), title: app_name.clone(), app: app_name, focused: false });
            continue;
        }
        for win_ref in &children {
            let Ok(win) = win_ref.as_accessible_proxy(bus).await else {
                continue;
            };
            let raw_title = win.name().await.unwrap_or_default();
            let title = if raw_title.trim().is_empty() { app_name.clone() } else { raw_title };
            let focused = win.get_state().await.map(|state| state.contains(State::Active)).unwrap_or(false);
            windows.push(DesktopWindow { id: object_ref_id(win_ref), title, app: app_name.clone(), focused });
        }
    }
    Ok(windows)
}

async fn snapshot_async(window_id: Option<String>, limit: SnapshotLimit) -> DesktopResult<DesktopSnapshot> {
    let conn = connect().await?;
    let bus = conn.connection();
    let (root_proxy, window) = match window_id {
        Some(id) => {
            let object_ref = object_ref_from_id(&id)?;
            let proxy = object_ref.into_accessible_proxy(bus).await.map_err(|e| DesktopError::WindowNotFound(format!("{id}: {e}")))?;
            let title = proxy.name().await.unwrap_or_default();
            let focused = proxy.get_state().await.map(|state| state.contains(State::Active) || state.contains(State::Focused)).unwrap_or(false);
            (proxy, DesktopWindow { id, title, app: String::new(), focused })
        }
        None => resolve_active_window(&conn).await?,
    };

    let mut counter = 0usize;
    let mut truncated = false;
    let root = build_node(bus, root_proxy, 0, &limit, &mut counter, &mut truncated).await;
    Ok(DesktopSnapshot { window, root, truncated })
}

async fn focus_async(window_id: String) -> DesktopResult<()> {
    let conn = connect().await?;
    let bus = conn.connection();
    let object_ref = object_ref_from_id(&window_id)?;
    let proxy = object_ref.as_accessible_proxy(bus).await.map_err(|e| DesktopError::WindowNotFound(format!("{window_id}: {e}")))?;
    let proxies = proxy.proxies().await.map_err(|e| DesktopError::Failed(format!("failed to get interfaces: {e}")))?;
    let component = proxies.component().await.map_err(|_| DesktopError::Failed("target does not support the Component interface; cannot focus".to_string()))?;
    component.grab_focus().await.map_err(|e| DesktopError::Failed(format!("grab_focus failed: {e}")))?;
    Ok(())
}

async fn resolve_active_window<'a>(conn: &'a AccessibilityConnection) -> DesktopResult<(AccessibleProxy<'a>, DesktopWindow)> {
    let bus = conn.connection();
    let root = conn.root_accessible_on_registry().await.map_err(|e| DesktopError::Failed(format!("failed to read AT-SPI root: {e}")))?;
    let apps = root.get_children().await.unwrap_or_default();
    let mut fallback: Option<(AccessibleProxy<'a>, String, String)> = None;

    for app_ref in &apps {
        let Ok(app) = app_ref.as_accessible_proxy(bus).await else {
            continue;
        };
        let app_name = app.name().await.unwrap_or_default();
        let children = app.get_children().await.unwrap_or_default();
        for win_ref in children {
            let id = object_ref_id(&win_ref);
            let Ok(win) = win_ref.into_accessible_proxy(bus).await else {
                continue;
            };
            let active = win.get_state().await.map(|state| state.contains(State::Active)).unwrap_or(false);
            if active {
                let title = win.name().await.unwrap_or_default();
                return Ok((win, DesktopWindow { id, title, app: app_name, focused: true }));
            }
            if fallback.is_none() {
                fallback = Some((win, app_name.clone(), id));
            }
        }
    }

    let Some((proxy, app_name, id)) = fallback else {
        return Err(DesktopError::WindowNotFound("no accessible window found".to_string()));
    };
    let title = proxy.name().await.unwrap_or_default();
    Ok((proxy, DesktopWindow { id, title, app: app_name, focused: false }))
}

fn build_node<'a>(
    bus: &'a Connection,
    proxy: AccessibleProxy<'a>,
    depth: usize,
    limit: &'a SnapshotLimit,
    counter: &'a mut usize,
    truncated: &'a mut bool,
) -> Pin<Box<dyn Future<Output = DesktopNode> + 'a>> {
    Box::pin(async move {
        let eid = *counter;
        *counter += 1;

        let role = role_name(&proxy).await;
        let name = proxy.name().await.unwrap_or_default();
        let value = value_of(&proxy).await;
        let rect = bounds_of(&proxy).await;
        let (enabled, focused) = state_flags(&proxy).await;

        let mut node = DesktopNode { eid, role, name, value, rect, enabled, focused, visible: true, pressable: false, children: Vec::new() };

        if depth >= limit.max_depth || *counter >= limit.max_nodes {
            if depth < limit.max_depth {
                *truncated = true;
            }
            return node;
        }

        let children = proxy.get_children().await.unwrap_or_default();
        for child_ref in children {
            if *counter >= limit.max_nodes {
                *truncated = true;
                break;
            }
            let Ok(child) = child_ref.as_accessible_proxy(bus).await else {
                continue;
            };
            let child_node = build_node(bus, child, depth + 1, limit, counter, truncated).await;
            node.children.push(child_node);
        }
        node
    })
}

async fn role_name(proxy: &AccessibleProxy<'_>) -> String {
    if let Ok(role) = proxy.get_role_name().await {
        if !role.trim().is_empty() {
            return role;
        }
    }
    proxy.get_role().await.map(|role| format!("{role:?}")).unwrap_or_else(|_| "unknown".to_string())
}

async fn value_of(proxy: &AccessibleProxy<'_>) -> Option<String> {
    let proxies = proxy.proxies().await.ok()?;
    if let Ok(text) = proxies.text().await {
        if let Ok(count) = text.character_count().await {
            if count > 0 {
                let n = count.min(VALUE_TEXT_MAX_CHARS);
                if let Ok(content) = text.get_text(0, n).await {
                    if !content.trim().is_empty() {
                        return Some(content);
                    }
                }
            }
        }
    }
    if let Ok(value) = proxies.value().await {
        if let Ok(current) = value.current_value().await {
            return Some(format!("{current}"));
        }
    }
    None
}

async fn bounds_of(proxy: &AccessibleProxy<'_>) -> Option<(i32, i32, i32, i32)> {
    let proxies = proxy.proxies().await.ok()?;
    let component = proxies.component().await.ok()?;
    let (x, y, width, height) = component.get_extents(CoordType::Screen).await.ok()?;
    if width <= 0 || height <= 0 {
        return None;
    }
    Some((x, y, width, height))
}

async fn state_flags(proxy: &AccessibleProxy<'_>) -> (bool, bool) {
    match proxy.get_state().await {
        Ok(state) => (state.contains(State::Enabled), state.contains(State::Focused)),
        Err(_) => (true, false),
    }
}

fn object_ref_id(object_ref: &ObjectRefOwned) -> String {
    format!("{}{}", object_ref.name_as_str().unwrap_or(""), object_ref.path_as_str())
}

fn object_ref_from_id(id: &str) -> DesktopResult<ObjectRefOwned> {
    let Some(path_start) = id.find('/') else {
        return Err(DesktopError::WindowNotFound(format!("invalid AT-SPI reference (expected :bus/path): {id}")));
    };
    let (name, path) = id.split_at(path_start);
    if name.is_empty() || path.is_empty() {
        return Err(DesktopError::WindowNotFound(format!("invalid AT-SPI reference (expected :bus/path): {id}")));
    }
    let name = UniqueName::try_from(name.to_string()).map_err(|e| DesktopError::WindowNotFound(format!("invalid bus name {name}: {e}")))?;
    let path = ObjectPath::try_from(path.to_string()).map_err(|e| DesktopError::WindowNotFound(format!("invalid object path {path}: {e}")))?;
    Ok(ObjectRef::new_owned(name, path))
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
