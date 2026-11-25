use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolConflictManager;
use crate::service::service_desktop::DesktopService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::service::service_desktop::{ActionSignal, SnapshotDiff};

const POLL_MS: u64 = 400;
const SETTLE_MS: u64 = 150;
const MAX_WAIT_MS: u64 = 8000;
const POLL_MAX_MS: u64 = 1600;
const MAX_POLLS: usize = 5;
const MAX_BATCH_ACTIONS: usize = 20;
const BLIND_WAIT_MAX_MS: u64 = 1200;
const SIGNAL_MAX_NODES: usize = 400;
const DIFF_MAX_LINES: usize = 14;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DesktopActionSpec {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub action: String,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default)]
    pub button: Option<String>,
    #[serde(default)]
    pub double: bool,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub keys: Option<String>,
    #[serde(default)]
    pub amount: Option<i32>,
    #[serde(default)]
    pub horizontal: bool,
    #[serde(default)]
    pub window_id: Option<String>,
    #[serde(default)]
    pub wait_ms: Option<u64>,
    #[serde(default)]
    pub eid: Option<usize>,
    #[serde(default)]
    pub submit: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolDesktopActionArgs {
    #[serde(flatten)]
    pub step: DesktopActionSpec,
    #[serde(default)]
    pub actions: Option<Vec<DesktopActionSpec>>,
    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolDesktopAction {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    desktop_service: DesktopService,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolDesktopAction {
    fn name(&self) -> &'static str {
        "ToolDesktopAction"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDesktopAction {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDesktopAction
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolDesktopAction {
    const NAME: &'static str = AiToolId::ToolDesktopAction.as_str();

    type Args = ToolDesktopActionArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "在本机图形桌面执行鼠标/键盘动作，是 desktop_snapshot『看屏』之后的『动手』步骤。**优先用 eid 而不是坐标**：press(eid) 走无障碍层直接按下控件，fill(eid,text) 直接把文本写进输入框（可带 submit=true 顺带回车），两者都会先把窗口激活、必要时自动滚到可见，比点坐标可靠得多，也不会因为窗口没在前台而点空。eid 来自 desktop_snapshot。**连续多步一次传 actions 数组**：最多 20 步顺序执行，整批只确认一次；某步失败即停止，已执行的不回滚。步上带 wait_ms（1–8000）时，做完会轮询无障碍树并把**界面变化（新增/消失了哪些元素）直接回给你**，不用再单独看屏。action：press(eid)、fill(eid,text,submit)、click(x,y，可 button/double，已含移动)、move、scroll(amount，horizontal)、type(text)、key(keys，如 ctrl+c / enter)、focus(window_id)、wait(只过闸门，必须带 wait_ms)。顶层 window_id 作为各项缺省。仅本机桌面，远程用 SSH/RDP。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "window_id": { "type": "string", "description": "缺省窗口 id；各项不传则用它" },
                "actions": {
                    "type": "array",
                    "description": "顺序多步，最多 20。能预知的连续操作优先用它",
                    "maxItems": MAX_BATCH_ACTIONS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "action": { "type": "string", "enum": ["press", "fill", "click", "move", "scroll", "type", "key", "focus", "wait"] },
                            "eid": { "type": "integer", "description": "desktop_snapshot 给出的元素 eid，press/fill 必填" },
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                            "button": { "type": "string" },
                            "double": { "type": "boolean" },
                            "text": { "type": "string" },
                            "submit": { "type": "boolean", "description": "fill 写完后回车提交" },
                            "keys": { "type": "string" },
                            "amount": { "type": "integer" },
                            "horizontal": { "type": "boolean" },
                            "window_id": { "type": "string" },
                            "wait_ms": { "type": "integer", "description": "1–8000。做完后等到 UI 变或超时，并返回界面变化" }
                        },
                        "required": ["action"]
                    }
                },
                "action": { "type": "string", "enum": ["press", "fill", "click", "move", "scroll", "type", "key", "focus", "wait"], "description": "单步动作。与 actions 二选一" },
                "eid": { "type": "integer" },
                "x": { "type": "integer" },
                "y": { "type": "integer" },
                "button": { "type": "string" },
                "double": { "type": "boolean" },
                "text": { "type": "string" },
                "submit": { "type": "boolean" },
                "keys": { "type": "string" },
                "amount": { "type": "integer" },
                "horizontal": { "type": "boolean" },
                "wait_ms": { "type": "integer" }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let specs = Self::resolve_actions(&args)?;
        let confirm_args = ToolDesktopActionArgs { step: DesktopActionSpec { window_id: args.step.window_id.clone(), ..Default::default() }, actions: Some(specs.clone()), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let total = specs.len();
        let mut done: Vec<String> = Vec::with_capacity(total);
        let mut items: Vec<WrapItem> = Vec::with_capacity(total);
        for (index, spec) in specs.iter().enumerate() {
            if run_context.cancel_token.is_cancelled() {
                return Err(AiToolError::Cancelled("desktop_action batch cancelled".to_string()));
            }
            match self.run_step(&run_context, spec).await {
                Ok(body) => {
                    done.push(body.clone());
                    items.push(WrapItem::wrap_item(Some(format!("===== {}/{} {} =====", index + 1, total, spec.action)), body));
                }
                Err(AiToolError::Cancelled(reason)) => return Err(AiToolError::Cancelled(reason)),
                Err(error) => {
                    return Err(AiToolError::Execution(format!("stopped at step {}/{} {}: {}\ncompleted:\n{}", index + 1, total, spec.action, error, done.join("\n"))));
                }
            }
        }
        context.insert_result(items);
        Ok(String::new())
    }
}

impl ToolDesktopAction {
    fn require_xy(spec: &DesktopActionSpec) -> Result<(i32, i32), AiToolError> {
        match (spec.x, spec.y) {
            (Some(x), Some(y)) => Ok((x, y)),
            _ => Err(AiToolError::Execution("this action requires x and y coordinates".to_string())),
        }
    }

    fn resolve_wait_ms(action: &str, wait_ms: Option<u64>) -> Result<Option<u64>, AiToolError> {
        if action == "wait" {
            let Some(ms) = wait_ms else {
                return Err(AiToolError::Execution("wait requires wait_ms".to_string()));
            };
            if ms == 0 || ms > MAX_WAIT_MS {
                return Err(AiToolError::Execution(format!("wait_ms must be 1–{}: {}", MAX_WAIT_MS, ms)));
            }
            return Ok(Some(ms));
        }
        if let Some(ms) = wait_ms {
            if ms == 0 || ms > MAX_WAIT_MS {
                return Err(AiToolError::Execution(format!("wait_ms must be 1–{}: {}", MAX_WAIT_MS, ms)));
            }
            return Ok(Some(ms));
        }
        Ok(None)
    }

    fn resolve_actions(args: &ToolDesktopActionArgs) -> Result<Vec<DesktopActionSpec>, AiToolError> {
        let top_window = args.step.window_id.as_deref().map(str::trim).filter(|value| !value.is_empty());
        if let Some(actions) = args.actions.as_ref().filter(|actions| !actions.is_empty()) {
            if actions.len() > MAX_BATCH_ACTIONS {
                return Err(AiToolError::Execution(format!("too many actions: {} exceeds {}", actions.len(), MAX_BATCH_ACTIONS)));
            }
            return actions.iter().map(|item| Self::normalize_spec(item, top_window)).collect();
        }
        Self::normalize_spec(&args.step, top_window).map(|spec| vec![spec])
    }

    fn normalize_spec(item: &DesktopActionSpec, top_window: Option<&str>) -> Result<DesktopActionSpec, AiToolError> {
        let action = item.action.trim().to_ascii_lowercase();
        if action.is_empty() {
            return Err(AiToolError::Execution("action is empty, or pass actions=[...] for batch".to_string()));
        }
        let wait_ms = Self::resolve_wait_ms(&action, item.wait_ms)?;
        let window_id = item.window_id.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_string).or_else(|| top_window.map(str::to_string));
        Ok(DesktopActionSpec {
            action,
            wait_ms,
            window_id,
            x: item.x,
            y: item.y,
            button: item.button.clone(),
            double: item.double,
            text: item.text.clone(),
            keys: item.keys.clone(),
            amount: item.amount,
            horizontal: item.horizontal,
            eid: item.eid,
            submit: item.submit,
        })
    }

    fn require_eid(spec: &DesktopActionSpec) -> Result<usize, AiToolError> {
        spec.eid.ok_or_else(|| AiToolError::Execution(format!("{} 需要 eid，先用 desktop_snapshot 拿到目标元素", spec.action)))
    }

    async fn run_step(&self, run_context: &AiRunContext, spec: &DesktopActionSpec) -> Result<String, AiToolError> {
        let baseline = match spec.wait_ms {
            Some(_) => Some(self.signal(spec.window_id.clone()).await?),
            None => None,
        };
        let acted = if spec.action == "wait" { String::new() } else { self.execute_one(spec).await? };
        let Some(wait_ms) = spec.wait_ms else {
            return Ok(acted);
        };
        let waited = self.wait_gate(run_context, spec.window_id.clone(), baseline, wait_ms).await?;
        if acted.is_empty() {
            return Ok(waited);
        }
        Ok(format!("{acted}; {waited}"))
    }

    async fn signal(&self, window_id: Option<String>) -> Result<ActionSignal, AiToolError> {
        self.desktop_service.action_signal(window_id, SIGNAL_MAX_NODES).await.map_err(|error| AiToolError::Execution(error.to_string()))
    }

    async fn wait_gate(&self, run_context: &AiRunContext, window_id: Option<String>, baseline: Option<ActionSignal>, wait_ms: u64) -> Result<String, AiToolError> {
        let Some(baseline) = baseline else {
            return Ok(String::new());
        };
        if baseline.interactive == 0 {
            let settle = wait_ms.min(BLIND_WAIT_MAX_MS);
            tokio::time::sleep(Duration::from_millis(settle)).await;
            return Ok(format!("等待 {settle}ms：这个窗口读不到任何可观测控件，轮询无意义已直接放行；先用 focus 把窗口切到前台再 desktop_snapshot"));
        }
        let mut diff;
        let mut polls = 0usize;
        let started = Instant::now();
        let limit = Duration::from_millis(wait_ms);
        let mut interval = Duration::from_millis(POLL_MS);
        loop {
            if run_context.cancel_token.is_cancelled() {
                return Err(AiToolError::Cancelled("desktop_action wait cancelled".to_string()));
            }
            let probe_started = Instant::now();
            let now = self.signal(window_id.clone()).await?;
            polls += 1;
            diff = now.diff_since(&baseline);
            if !diff.is_empty() {
                break;
            }
            if polls >= MAX_POLLS {
                break;
            }
            if started.elapsed() >= limit {
                break;
            }
            let rest = interval.saturating_sub(probe_started.elapsed());
            tokio::time::sleep(rest).await;
            interval = (interval * 2).min(Duration::from_millis(POLL_MAX_MS));
        }
        let elapsed = started.elapsed().as_millis();
        tokio::time::sleep(Duration::from_millis(SETTLE_MS)).await;
        Ok(Self::render_diff(&diff, elapsed, polls))
    }

    fn render_diff(diff: &SnapshotDiff, elapsed: u128, polls: usize) -> String {
        if diff.is_empty() {
            return format!("等待 {elapsed}ms 轮询{polls}次：界面没有任何变化。这一步很可能没生效——不要原样重试，改用 press(eid) 走无障碍层，或先 desktop_snapshot 确认当前状态");
        }
        let mut out = format!("等待 {elapsed}ms 轮询{polls}次：界面已变化（{}）\n", diff.summary());
        let shown = diff.added.len().min(DIFF_MAX_LINES);
        if shown > 0 {
            out.push_str(&format!("新出现的元素（共{}个，列前{shown}个）：\n", diff.added.len()));
            for line in diff.added.iter().take(shown) {
                out.push_str(line);
                out.push('\n');
            }
        }
        if diff.removed.is_empty() {
            return out;
        }
        out.push_str(&format!("消失了 {} 个元素\n", diff.removed.len()));
        out
    }

    async fn execute_one(&self, spec: &DesktopActionSpec) -> Result<String, AiToolError> {
        let fail = |error: anyhow::Error| AiToolError::Execution(error.to_string());
        match spec.action.as_str() {
            "press" => {
                let eid = Self::require_eid(spec)?;
                self.activate(spec).await?;
                let moved = self.desktop_service.scroll_into_view(eid).await.map_err(fail)?;
                let pressed = self.desktop_service.press_element(eid).await;
                if let Ok(action) = pressed {
                    return Ok(format!("已按下 eid={eid}（{action}）"));
                }
                let (x, y) = moved.center();
                self.desktop_service.click(x, y, spec.button.clone().unwrap_or_default(), spec.double).await.map_err(fail)?;
                Ok(format!("eid={eid} 不支持无障碍按下，已改点它的中心 ({x},{y})"))
            }
            "fill" => {
                let eid = Self::require_eid(spec)?;
                let text = spec.text.clone().ok_or_else(|| AiToolError::Execution("fill requires text".to_string()))?;
                self.activate(spec).await?;
                let moved = self.desktop_service.scroll_into_view(eid).await.map_err(fail)?;
                let _ = self.desktop_service.focus_element(eid).await;
                let written = self.desktop_service.set_element_text(eid, text.clone()).await;
                let how = match written {
                    Ok(()) => "已直接写入".to_string(),
                    Err(_) => {
                        let (x, y) = moved.center();
                        self.desktop_service.click(x, y, String::new(), false).await.map_err(fail)?;
                        self.desktop_service.key_combo("cmd+a".to_string()).await.map_err(fail)?;
                        self.desktop_service.type_text(text.clone()).await.map_err(fail)?;
                        format!("不可直接写值，已点 ({x},{y}) 全选后键入")
                    }
                };
                if !spec.submit {
                    return Ok(format!("eid={eid} {how} {} 个字符", text.chars().count()));
                }
                self.desktop_service.key_combo("enter".to_string()).await.map_err(fail)?;
                Ok(format!("eid={eid} {how} {} 个字符并回车", text.chars().count()))
            }
            "click" => {
                let (x, y) = Self::require_xy(spec)?;
                self.activate(spec).await?;
                self.desktop_service.click(x, y, spec.button.clone().unwrap_or_default(), spec.double).await.map_err(fail)?;
                Ok(format!("已点击 ({x},{y})"))
            }
            "move" => {
                let (x, y) = Self::require_xy(spec)?;
                self.desktop_service.move_mouse(x, y).await.map_err(fail)?;
                Ok(format!("已移动到 ({x},{y})"))
            }
            "scroll" => {
                let amount = spec.amount.ok_or_else(|| AiToolError::Execution("scroll requires amount".to_string()))?;
                self.desktop_service.scroll(amount, spec.horizontal).await.map_err(fail)?;
                Ok(format!("已滚动 {amount}"))
            }
            "type" => {
                let text = spec.text.clone().ok_or_else(|| AiToolError::Execution("type requires text".to_string()))?;
                self.activate(spec).await?;
                self.desktop_service.type_text(text.clone()).await.map_err(fail)?;
                Ok(format!("已输入 {} 个字符", text.chars().count()))
            }
            "key" => {
                let keys = spec.keys.clone().ok_or_else(|| AiToolError::Execution("key requires keys".to_string()))?;
                self.activate(spec).await?;
                self.desktop_service.key_combo(keys.clone()).await.map_err(fail)?;
                Ok(format!("已按下 {keys}"))
            }
            "focus" => {
                let window_id = spec.window_id.clone().ok_or_else(|| AiToolError::Execution("focus requires window_id".to_string()))?;
                self.desktop_service.focus_window(window_id.clone()).await.map_err(fail)?;
                Ok(format!("已置前窗口 {window_id}"))
            }
            other => Err(AiToolError::Execution(format!("unknown action: {other}"))),
        }
    }

    async fn activate(&self, spec: &DesktopActionSpec) -> Result<(), AiToolError> {
        self.desktop_service.activate_window(spec.window_id.clone()).await.map_err(|error| AiToolError::Execution(error.to_string()))
    }
}
