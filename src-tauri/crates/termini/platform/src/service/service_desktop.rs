use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
#[cfg(target_os = "macos")]
use desktop_apple as desktop_impl;
#[cfg(target_os = "linux")]
use desktop_linux as desktop_impl;
#[cfg(target_os = "windows")]
use desktop_windows as desktop_impl;
use crate::util::clipboard_file_util;
use crate::base::base_app::{AppContext, BaseApp};
use desktop_core::{DesktopWindow, ElementBox, SnapshotLimit};
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
pub use desktop_core::{ActionSignal, DesktopNode, DesktopSnapshot, RenderCaps, RenderStat, SnapshotDiff};

const SCROLL_MAX_STEPS: usize = 30;
const SCROLL_NEAR_PX: i32 = 80;
const SCROLL_MID_PX: i32 = 250;
const SCROLL_PAUSE_MS: u64 = 120;

#[derive(Clone, Service)]
pub struct DesktopService {}

#[async_trait]
impl BaseApp for DesktopService {
    fn name(&self) -> &'static str {
        "DesktopService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Ok(())
    }
}

impl DesktopService {
    pub async fn list_windows(&self) -> Result<Vec<DesktopWindow>> {
        summer::tracing::info!("[desktop] list_windows");
        let result = tokio::task::spawn_blocking(|| desktop_impl::list_windows().map_err(|e| anyhow!(e))).await?;
        match &result {
            Ok(windows) => summer::tracing::info!("[desktop] list_windows ok count={}", windows.len()),
            Err(error) => summer::tracing::warn!("[desktop] list_windows failed: {error:#}"),
        }
        result
    }

    pub async fn snapshot(&self, window_id: Option<String>, max_nodes: usize) -> Result<DesktopSnapshot> {
        let limit = SnapshotLimit { max_nodes, probe_hidden: true, ..SnapshotLimit::default() };
        self.snapshot_with(window_id, limit).await
    }

    async fn snapshot_with(&self, window_id: Option<String>, limit: SnapshotLimit) -> Result<DesktopSnapshot> {
        summer::tracing::info!("[desktop] snapshot window_id={:?} max_nodes={} max_depth={} probe_hidden={}", window_id, limit.max_nodes, limit.max_depth, limit.probe_hidden);
        let result = tokio::task::spawn_blocking(move || desktop_impl::snapshot(window_id.as_deref(), limit).map_err(|e| anyhow!(e))).await?;
        match &result {
            Ok(snapshot) => summer::tracing::info!("[desktop] snapshot ok id={} title={} truncated={}", snapshot.window.id, snapshot.window.title, snapshot.truncated),
            Err(error) => summer::tracing::warn!("[desktop] snapshot failed: {error:#}"),
        }
        result
    }

    pub async fn action_signal(&self, window_id: Option<String>, max_nodes: usize) -> Result<ActionSignal> {
        let limit = SnapshotLimit { max_nodes, probe_hidden: false, ..SnapshotLimit::default() };
        let snapshot = self.snapshot_with(window_id, limit).await?;
        Ok(snapshot.action_signal())
    }

    pub async fn focus_window(&self, window_id: String) -> Result<()> {
        summer::tracing::info!("[desktop] focus_window window_id={window_id}");
        let result = tokio::task::spawn_blocking(move || desktop_impl::focus_window(&window_id).map_err(|e| anyhow!(e))).await?;
        match &result {
            Ok(()) => summer::tracing::info!("[desktop] focus_window ok"),
            Err(error) => summer::tracing::warn!("[desktop] focus_window failed: {error:#}"),
        }
        result
    }

    pub async fn activate_window(&self, window_id: Option<String>) -> Result<()> {
        summer::tracing::info!("[desktop] activate_window window_id={window_id:?}");
        tokio::task::spawn_blocking(move || desktop_impl::activate_window(window_id.as_deref()).map_err(|e| anyhow!(e))).await?
    }

    pub async fn press_element(&self, eid: usize) -> Result<String> {
        summer::tracing::info!("[desktop] press_element eid={eid}");
        tokio::task::spawn_blocking(move || desktop_impl::press_element(eid).map_err(|e| anyhow!(e))).await?
    }

    pub async fn focus_element(&self, eid: usize) -> Result<()> {
        summer::tracing::info!("[desktop] focus_element eid={eid}");
        tokio::task::spawn_blocking(move || desktop_impl::focus_element(eid).map_err(|e| anyhow!(e))).await?
    }

    pub async fn set_element_text(&self, eid: usize, text: String) -> Result<()> {
        summer::tracing::info!("[desktop] set_element_text eid={eid} len={}", text.chars().count());
        tokio::task::spawn_blocking(move || desktop_impl::set_element_text(eid, &text).map_err(|e| anyhow!(e))).await?
    }

    pub async fn element_box(&self, eid: usize) -> Result<ElementBox> {
        tokio::task::spawn_blocking(move || desktop_impl::element_box(eid).map_err(|e| anyhow!(e))).await?
    }

    pub async fn scroll_into_view(&self, eid: usize) -> Result<ElementBox> {
        let mut current = self.element_box(eid).await?;
        let mut step = 0usize;
        while !current.visible && step < SCROLL_MAX_STEPS {
            let distance = current.distance_below();
            if distance == 0 {
                break;
            }
            if let Some((wx, wy, ww, wh)) = current.window {
                self.move_mouse(wx + ww / 2, wy + wh / 2).await?;
            }
            self.scroll(scroll_lines(distance), false).await?;
            tokio::time::sleep(std::time::Duration::from_millis(SCROLL_PAUSE_MS)).await;
            current = self.element_box(eid).await?;
            step += 1;
        }
        summer::tracing::info!("[desktop] scroll_into_view eid={eid} steps={step} visible={}", current.visible);
        Ok(current)
    }

    pub async fn click(&self, x: i32, y: i32, button: String, double: bool) -> Result<()> {
        summer::tracing::info!("[desktop] click x={x} y={y} button={button} double={double}");
        Self::run_injected(move |enigo| {
            let button = parse_button(&button)?;
            enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| anyhow!("failed to move mouse: {e}"))?;
            enigo.button(button, Direction::Click).map_err(|e| anyhow!("click failed: {e}"))?;
            if double {
                std::thread::sleep(std::time::Duration::from_millis(60));
                enigo.button(button, Direction::Click).map_err(|e| anyhow!("second click failed: {e}"))?;
            }
            Ok(())
        })
        .await
    }

    pub async fn move_mouse(&self, x: i32, y: i32) -> Result<()> {
        summer::tracing::info!("[desktop] move_mouse x={x} y={y}");
        Self::run_injected(move |enigo| enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| anyhow!("failed to move mouse: {e}"))).await
    }

    pub async fn scroll(&self, amount: i32, horizontal: bool) -> Result<()> {
        summer::tracing::info!("[desktop] scroll amount={amount} horizontal={horizontal}");
        Self::run_injected(move |enigo| {
            let axis = if horizontal { Axis::Horizontal } else { Axis::Vertical };
            enigo.scroll(amount, axis).map_err(|e| anyhow!("scroll failed: {e}"))
        })
        .await
    }

    pub async fn type_text(&self, text: String) -> Result<()> {
        summer::tracing::info!("[desktop] type_text len={}", text.chars().count());
        Self::run_injected(move |enigo| enigo.text(&text).map_err(|e| anyhow!("failed to type text: {e}"))).await
    }

    pub async fn key_combo(&self, combo: String) -> Result<()> {
        summer::tracing::info!("[desktop] key_combo combo={combo}");
        Self::run_injected(move |enigo| {
            let keys = parse_combo(&combo)?;
            let (main, modifiers) = keys.split_last().ok_or_else(|| anyhow!("empty key combo"))?;
            for key in modifiers {
                enigo.key(*key, Direction::Press).map_err(|e| anyhow!("failed to press modifier key: {e}"))?;
            }
            let result = enigo.key(*main, Direction::Click).map_err(|e| anyhow!("key press failed: {e}"));
            for key in modifiers.iter().rev() {
                let _ = enigo.key(*key, Direction::Release);
            }
            result
        })
        .await
    }

    async fn run_injected<F>(action: F) -> Result<()>
    where
        F: FnOnce(&mut Enigo) -> Result<()> + Send + 'static,
    {
        tokio::task::spawn_blocking(move || {
            clipboard_file_util::set_input_injection(true);
            let result = run_on_ui_thread(|| {
                let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow!("failed to initialize input simulation: {e}"))?;
                action(&mut enigo)
            });
            std::thread::sleep(std::time::Duration::from_millis(80));
            clipboard_file_util::set_input_injection(false);
            match &result {
                Ok(()) => summer::tracing::info!("[desktop] inject ok"),
                Err(error) => summer::tracing::warn!("[desktop] inject failed: {error:#}"),
            }
            result
        })
        .await?
    }
}

fn run_on_ui_thread<T, F>(work: F) -> T
where
    T: Send,
    F: FnOnce() -> T + Send,
{
    #[cfg(target_os = "macos")]
    {
        return dispatch::Queue::main().exec_sync(work);
    }
    #[cfg(not(target_os = "macos"))]
    work()
}

fn scroll_lines(distance: i32) -> i32 {
    let magnitude = match distance.abs() {
        gap if gap < SCROLL_NEAR_PX => 1,
        gap if gap < SCROLL_MID_PX => 2,
        _ => 3,
    };
    if distance < 0 {
        return -magnitude;
    }
    magnitude
}

fn parse_button(raw: &str) -> Result<Button> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "left" => Ok(Button::Left),
        "right" => Ok(Button::Right),
        "middle" => Ok(Button::Middle),
        other => Err(anyhow!("unknown mouse button: {other}")),
    }
}

fn parse_combo(raw: &str) -> Result<Vec<Key>> {
    let mut keys = Vec::new();
    for part in raw.split('+') {
        let token = part.trim().to_ascii_lowercase();
        if token.is_empty() {
            continue;
        }
        keys.push(parse_key(&token)?);
    }
    if keys.is_empty() {
        return Err(anyhow!("empty key combo: {raw}"));
    }
    Ok(keys)
}

fn parse_key(token: &str) -> Result<Key> {
    let key = match token {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" | "option" => Key::Alt,
        "meta" | "win" | "cmd" | "command" | "super" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        other => {
            let mut chars = other.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                return Err(anyhow!("unknown key: {other}"));
            };
            Key::Unicode(ch)
        }
    };
    Ok(key)
}
