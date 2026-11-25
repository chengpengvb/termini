use serde::Serialize;

pub const REQ_REF_PREFIX: &str = "@[";
pub const REQ_REF_FILE: i16 = 0;
pub const REQ_REF_DIR: i16 = 1;

pub const REQ_REF_KIND_DEVICE: &str = "device";
pub const REQ_REF_KIND_GROUP: &str = "group";
pub const REQ_REF_KIND_LOCAL_PATH: &str = "localpath";
pub const REQ_REF_KIND_SKILL: &str = "skill";
pub const REQ_REF_KIND_MCP_PROMPT: &str = "mcpprompt";
pub const REQ_REF_KIND_MCP_TOOL: &str = "mcptool";
pub const REQ_REF_KIND_MCP_SERVER: &str = "mcpserver";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReqRefKind {
    Device,
    Group,
    LocalPath,
    Skill,
    McpPrompt,
    McpTool,
    McpServer,
}

impl ReqRefKind {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            REQ_REF_KIND_DEVICE => Some(Self::Device),
            REQ_REF_KIND_GROUP => Some(Self::Group),
            REQ_REF_KIND_LOCAL_PATH => Some(Self::LocalPath),
            REQ_REF_KIND_SKILL => Some(Self::Skill),
            REQ_REF_KIND_MCP_PROMPT => Some(Self::McpPrompt),
            REQ_REF_KIND_MCP_TOOL => Some(Self::McpTool),
            REQ_REF_KIND_MCP_SERVER => Some(Self::McpServer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReqRefToken {
    pub kind: ReqRefKind,
    pub id: String,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReqRefDevice {
    pub device_id: String,
    pub name: String,
    pub host: String,
    pub os_type: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReqRefGroup {
    pub group_id: String,
    pub name: String,
    pub device_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReqRefPath {
    pub path: String,
    pub name: String,
    pub entry_kind: i16,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReqRefMcpServer {
    pub server_id: String,
    pub name: String,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReqRefTargets {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ReqRefGroup>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<ReqRefDevice>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub local_paths: Vec<ReqRefPath>,
}
