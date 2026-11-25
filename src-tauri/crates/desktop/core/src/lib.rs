use serde::{Deserialize, Serialize};

const NAME_MAX_CHARS: usize = 60;
const VALUE_MAX_CHARS: usize = 40;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWindow {
    pub id: String,
    pub title: String,
    pub app: String,
    pub focused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNode {
    pub eid: usize,
    pub role: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<(i32, i32, i32, i32)>,
    pub enabled: bool,
    pub focused: bool,
    pub visible: bool,
    pub pressable: bool,
    pub children: Vec<DesktopNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSnapshot {
    pub window: DesktopWindow,
    pub root: DesktopNode,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapshotLimit {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub probe_hidden: bool,
}

#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub key: String,
    pub line: String,
}

#[derive(Debug, Clone, Default)]
pub struct ActionSignal {
    pub entries: Vec<DiffEntry>,
    pub interactive: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ElementBox {
    pub rect: (i32, i32, i32, i32),
    pub window: Option<(i32, i32, i32, i32)>,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderCaps {
    pub interactive: usize,
    pub text: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStat {
    pub interactive: usize,
    pub text: usize,
    pub skipped: usize,
}

impl Default for SnapshotLimit {
    fn default() -> Self {
        Self { max_depth: 24, max_nodes: 600, probe_hidden: false }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DesktopError {
    #[error("desktop element tree is not supported on this platform: {0}")]
    Unsupported(&'static str),

    #[error("window not found: {0}")]
    WindowNotFound(String),

    #[error("element not found: {0}")]
    ElementNotFound(String),

    #[error("desktop automation failed: {0}")]
    Failed(String),
}

pub type DesktopResult<T> = Result<T, DesktopError>;

impl SnapshotDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }

    pub fn summary(&self) -> String {
        format!("新增{} 消失{}", self.added.len(), self.removed.len())
    }
}

impl ElementBox {
    pub fn center(&self) -> (i32, i32) {
        let (x, y, width, height) = self.rect;
        (x + width / 2, y + height / 2)
    }

    pub fn distance_below(&self) -> i32 {
        let Some((_, wy, _, wh)) = self.window else {
            return 0;
        };
        let (_, cy) = self.center();
        if cy > wy + wh {
            return cy - (wy + wh);
        }
        if cy < wy {
            return cy - wy;
        }
        0
    }
}

impl DesktopNode {
    pub fn collect_rects(&self, out: &mut Vec<(usize, (i32, i32, i32, i32))>) {
        if let Some(rect) = self.rect {
            out.push((self.eid, rect));
        }
        for child in &self.children {
            child.collect_rects(out);
        }
    }

    pub fn labeled(&self) -> bool {
        !self.name.is_empty() || self.value.as_deref().is_some_and(|text| !text.is_empty())
    }

    pub fn is_interactive_role(&self) -> bool {
        const INTERACTIVE: &[&str] = &[
            "Button",
            "Edit",
            "TextField",
            "TextArea",
            "CheckBox",
            "RadioButton",
            "ComboBox",
            "PopUpButton",
            "MenuItem",
            "MenuButton",
            "TabItem",
            "Tab",
            "ListItem",
            "Hyperlink",
            "Link",
            "TreeItem",
            "Slider",
            "SplitButton",
            "ToggleButton",
            "Spinner",
            "SearchField",
            "Cell",
            "Row",
        ];
        INTERACTIVE.iter().any(|role| self.role == *role)
    }

    pub fn is_actionable(&self) -> bool {
        self.pressable || self.is_interactive_role()
    }

    pub fn renderable(&self) -> bool {
        if !self.visible {
            return false;
        }
        if self.rect.is_none() {
            return false;
        }
        if self.labeled() {
            return true;
        }
        self.is_actionable()
    }

    pub fn interactive_count(&self) -> usize {
        let own = usize::from(self.renderable());
        own + self.children.iter().map(|child| child.interactive_count()).sum::<usize>()
    }

    pub fn center(&self) -> Option<(i32, i32)> {
        let (x, y, width, height) = self.rect?;
        Some((x + width / 2, y + height / 2))
    }

    pub fn line(&self) -> String {
        let mut out = String::new();
        out.push('[');
        out.push_str(&self.eid.to_string());
        out.push_str("] ");
        out.push_str(&self.role);
        if !self.name.is_empty() {
            out.push_str(" \"");
            out.push_str(&truncate_chars(&self.name, NAME_MAX_CHARS));
            out.push('"');
        }
        if let Some(value) = self.value.as_deref().filter(|text| !text.is_empty()) {
            out.push_str(" =");
            out.push_str(&truncate_chars(value, VALUE_MAX_CHARS));
        }
        if !self.enabled {
            out.push_str(" disabled");
        }
        if self.focused {
            out.push_str(" focused");
        }
        if self.pressable {
            out.push_str(" 可点");
        }
        if let Some((cx, cy)) = self.center() {
            out.push_str(&format!(" @({cx},{cy})"));
        }
        out
    }

    pub fn diff_key(&self) -> String {
        format!("{}\t{}\t{}", self.role, self.name, self.value.as_deref().unwrap_or_default())
    }

    pub fn collect_entries(&self, out: &mut Vec<DiffEntry>) {
        if self.renderable() {
            out.push(DiffEntry { key: self.diff_key(), line: self.line() });
        }
        for child in &self.children {
            child.collect_entries(out);
        }
    }

    pub fn render_visible(&self, out: &mut String, caps: &RenderCaps, stat: &mut RenderStat) {
        self.append_visible(out, caps, stat);
        for child in &self.children {
            child.render_visible(out, caps, stat);
        }
    }

    fn append_visible(&self, out: &mut String, caps: &RenderCaps, stat: &mut RenderStat) {
        if !self.renderable() {
            return;
        }
        let interactive = self.is_actionable();
        let used = if interactive { stat.interactive } else { stat.text };
        let cap = if interactive { caps.interactive } else { caps.text };
        if used >= cap {
            stat.skipped += 1;
            return;
        }
        if interactive {
            stat.interactive += 1;
        } else {
            stat.text += 1;
        }
        out.push_str(&self.line());
        out.push('\n');
    }

    pub fn render_text(&self, out: &mut String, depth: usize) {
        for _ in 0..depth {
            out.push_str("  ");
        }
        out.push_str(&self.line());
        if let Some((x, y, w, h)) = self.rect {
            out.push_str(&format!(" rect=({x},{y},{w},{h})"));
        }
        if !self.visible {
            out.push_str(" offscreen");
        }
        out.push('\n');
        for child in &self.children {
            child.render_text(out, depth + 1);
        }
    }

    pub fn find_by_eid(&self, eid: usize) -> Option<&DesktopNode> {
        if self.eid == eid {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find_by_eid(eid))
    }

    pub fn collect_query(&self, needle: &str, out: &mut Vec<String>) {
        let name = self.name.to_lowercase();
        let value = self.value.as_deref().unwrap_or_default().to_lowercase();
        let hit = name.contains(needle) || value.contains(needle);
        if hit && self.rect.is_some() {
            let mut line = self.line();
            if !self.visible {
                line.push_str(" offscreen");
            }
            out.push(line);
        }
        for child in &self.children {
            child.collect_query(needle, out);
        }
    }

    pub fn find_by_text(&self, needle: &str) -> Option<&DesktopNode> {
        let hit = self.labeled() && (self.name.contains(needle) || self.value.as_deref().is_some_and(|text| text.contains(needle)));
        if hit {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find_by_text(needle))
    }
}

impl DesktopSnapshot {
    pub fn action_signal(&self) -> ActionSignal {
        let mut entries = Vec::new();
        self.root.collect_entries(&mut entries);
        ActionSignal { interactive: entries.len(), entries }
    }
}

impl ActionSignal {
    pub fn diff_since(&self, before: &ActionSignal) -> SnapshotDiff {
        SnapshotDiff { added: missing_lines(&self.entries, &before.entries), removed: missing_lines(&before.entries, &self.entries) }
    }
}

fn missing_lines(source: &[DiffEntry], other: &[DiffEntry]) -> Vec<String> {
    let mut pool: Vec<&str> = other.iter().map(|entry| entry.key.as_str()).collect();
    let mut out = Vec::new();
    for entry in source {
        let found = pool.iter().position(|key| *key == entry.key.as_str());
        if let Some(index) = found {
            pool.swap_remove(index);
            continue;
        }
        out.push(entry.line.clone());
    }
    out
}

fn truncate_chars(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    let head: String = input.chars().take(max).collect();
    format!("{head}…")
}
