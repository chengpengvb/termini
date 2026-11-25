use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::service::service_desktop::DesktopService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::service::service_desktop::{DesktopNode, DesktopSnapshot, RenderCaps, RenderStat};

const RENDER_MAX_LINES: usize = 120;
const SNAPSHOT_MAX_NODES: usize = 900;
const SNAPSHOT_MAX_RESULT_TOKENS: usize = 3000;
const CAP_INTERACTIVE: usize = 90;
const CAP_TEXT: usize = 60;
const QUERY_MAX_HITS: usize = 40;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolDesktopSnapshotArgs {
    #[serde(default)]
    pub window_id: Option<String>,

    #[serde(default)]
    pub list_windows: bool,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub query: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolDesktopSnapshot {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    desktop_service: DesktopService,
}

#[async_trait]
impl BaseApp for ToolDesktopSnapshot {
    fn name(&self) -> &'static str {
        "ToolDesktopSnapshot"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDesktopSnapshot {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDesktopSnapshot
    }

    fn dedup_in_run(&self) -> bool {
        false
    }

    fn supersede_in_run(&self) -> bool {
        true
    }

    fn persist(&self) -> bool {
        true
    }

    fn max_result_tokens(&self) -> usize {
        SNAPSHOT_MAX_RESULT_TOKENS
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

impl Tool for ToolDesktopSnapshot {
    const NAME: &'static str = AiToolId::ToolDesktopSnapshot.as_str();

    type Args = ToolDesktopSnapshotArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "读取本机图形桌面当前**屏幕上看得见**的界面结构（无障碍元素树），是 desktop_action 操作前的『看屏』步骤。list_windows=true 只返回顶层窗口清单（含 window_id）；否则返回指定窗口（或当前前台窗口）里视口内的控件，每行 [eid] role \"name\" =value 和点击中心 @(x,y)，带『可点』标记的是无障碍层面确认可以按下的。**拿到 eid 后优先用 desktop_action 的 press/fill 按 eid 操作，比按坐标点可靠得多**。屏幕外的元素默认不列；要找当前没显示出来的东西（比如某首歌、某个列表项）传 query 关键字，会在整棵树里检索并标出 offscreen，再用 press 即可（会自动滚到可见再点）。元素多时按页返回，续读传 offset，禁止同参数重复读同一页。仅本机桌面有效，远程设备走 SSH/RDP。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "window_id": {
                    "type": "string",
                    "description": "目标窗口 id(来自 list_windows 结果)；省略则读取当前前台窗口"
                },
                "list_windows": {
                    "type": "boolean",
                    "description": "为 true 时只返回顶层窗口清单，不展开元素树"
                },
                "offset": {
                    "type": "integer",
                    "description": "可操作元素列表的起始序号，省略为 0；返回里会给出总个数和下一页 offset"
                },
                "limit": {
                    "type": "integer",
                    "description": "本次返回的元素条数，省略按默认页大小"
                },
                "query": {
                    "type": "string",
                    "description": "关键字检索整棵树（含屏幕外元素），用于找当前没显示出来的目标；命中行会标 offscreen"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let body = if args.list_windows {
            let windows = self.desktop_service.list_windows().await.map_err(|e| AiToolError::Execution(e.to_string()))?;
            if windows.is_empty() {
                "no visible top-level windows found".to_string()
            } else {
                let mut out = String::from("顶层窗口（window_id | 标题 | 应用 | 是否前台）：\n");
                for window in windows {
                    out.push_str(&format!("{} | {} | {} | {}\n", window.id, window.title, window.app, if window.focused { "前台" } else { "" }));
                }
                out
            }
        } else {
            let snapshot = self.desktop_service.snapshot(args.window_id, SNAPSHOT_MAX_NODES).await.map_err(|e| AiToolError::Execution(e.to_string()))?;
            let mut out = String::new();
            out.push_str(&format!("窗口: {} (应用={} id={})\n", snapshot.window.title, snapshot.window.app, snapshot.window.id));
            if let Some((x, y, w, h)) = snapshot.root.rect {
                out.push_str(&format!("窗口区域: rect=({x},{y},{w},{h})\n"));
            }
            let needle = args.query.as_deref().map(str::trim).filter(|text| !text.is_empty());
            if let Some(needle) = needle {
                Self::render_query(&snapshot.root, needle, &mut out);
                out
            } else {
                Self::render_viewport(&snapshot, args.offset, args.limit, &mut out);
                out
            }
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolDesktopSnapshot {
    fn render_query(root: &DesktopNode, needle: &str, out: &mut String) {
        let mut hits = Vec::new();
        root.collect_query(&needle.to_lowercase(), &mut hits);
        if hits.is_empty() {
            out.push_str(&format!("检索 \"{needle}\"：整棵树里没有匹配的元素。可能还没加载出来，先滚动或触发搜索再查；也可以去掉 query 看看当前视口有什么。\n"));
            return;
        }
        let shown = hits.len().min(QUERY_MAX_HITS);
        out.push_str(&format!("检索 \"{needle}\"：命中 {} 个，列出前 {shown} 个（带 offscreen 的在屏幕外，press 它会自动滚到可见再点）：\n", hits.len()));
        for line in hits.iter().take(shown) {
            out.push_str(line);
            out.push('\n');
        }
    }

    fn render_viewport(snapshot: &DesktopSnapshot, offset: Option<usize>, limit: Option<usize>, out: &mut String) {
        let caps = RenderCaps { interactive: CAP_INTERACTIVE, text: CAP_TEXT };
        let mut stat = RenderStat::default();
        let mut tree = String::new();
        snapshot.root.render_visible(&mut tree, &caps, &mut stat);
        let lines: Vec<&str> = tree.lines().collect();
        let start = offset.unwrap_or(0).min(lines.len());
        let page = limit.unwrap_or(RENDER_MAX_LINES).clamp(1, RENDER_MAX_LINES);
        let end = (start + page).min(lines.len());
        out.push_str(&format!("视口内元素 共{}个，本页第{start}-{end}个（[eid] role \"name\" =value @(点击中心x,y)）：\n", lines.len()));
        for line in &lines[start..end] {
            out.push_str(line);
            out.push('\n');
        }
        if lines.is_empty() {
            out.push_str("（这个窗口在视口内读不到任何控件：先用 desktop_action 的 focus 把它切到前台再读一次；仍为空就用 list_windows 换 window_id，不要凭猜测点坐标。）\n");
        }
        if end < lines.len() {
            out.push_str(&format!("（还有 {} 个没列出，续读把 offset 传 {end}，不要改其他参数重复读同一页）\n", lines.len() - end));
        }
        if stat.skipped > 0 {
            out.push_str(&format!("（另有 {} 个同类控件超出上限没统计，要找具体目标用 query 关键字检索）\n", stat.skipped));
        }
        if snapshot.truncated {
            out.push_str("（元素树到节点上限，深层控件可能没读全；要找具体目标用 query 检索）\n");
        }
    }
}
