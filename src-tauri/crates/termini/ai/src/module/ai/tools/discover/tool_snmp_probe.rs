use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use std::str::FromStr;
use std::sync::OnceLock;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use protocol_snmp::{Oid, SnmpClient, SnmpValue, VarBind};
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use termini_collect::collect::collector::r#impl::collector_snmp::{CollectorSnmp, SNMP_OP_GET, SNMP_OP_WALK};

pub const SNMP_OP_SUGGEST: &str = "suggest";
pub const SNMP_PROBE_ROWS_MAX: usize = 200;
pub const SNMP_PROBE_ROWS_DEFAULT: usize = 50;

const OID_SYS_DESCR: &str = "1.3.6.1.2.1.1.1.0";
const OID_SYS_OBJECT_ID: &str = "1.3.6.1.2.1.1.2.0";
const ENTERPRISE_PREFIX: [u32; 6] = [1, 3, 6, 1, 4, 1];

const VENDOR_OIDS_JSON: &str = include_str!("../../../../../resource/snmp/vendor_oids.json");

static VENDOR_KNOWLEDGE: OnceLock<VendorKnowledge> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct VendorKnowledge {
    generic: Vec<KnowledgeIndicator>,
    vendors: Vec<KnowledgeVendor>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct KnowledgeVendor {
    enterprise: u32,
    name: String,
    indicators: Vec<KnowledgeIndicator>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KnowledgeIndicator {
    name: String,
    candidates: Vec<KnowledgeCandidate>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KnowledgeCandidate {
    oid: String,
    op: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    desc: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    expr: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    paired: Option<bool>,
}

fn vendor_knowledge() -> Result<&'static VendorKnowledge, AiToolError> {
    if let Some(knowledge) = VENDOR_KNOWLEDGE.get() {
        return Ok(knowledge);
    }
    let parsed: VendorKnowledge = serde_json::from_str(VENDOR_OIDS_JSON).map_err(|error| AiToolError::Execution(format!("failed to parse vendor OID knowledge base: {}", error)))?;
    let _ = VENDOR_KNOWLEDGE.set(parsed);
    Ok(VENDOR_KNOWLEDGE.get().expect("vendor knowledge just set"))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolSnmpProbeArgs {
    pub device_id: String,
    pub op: String,

    #[serde(default)]
    pub oids: Vec<String>,

    #[serde(default)]
    pub max_rows: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolSnmpProbeRow {
    pub oid: String,
    pub value_type: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolSnmpProbeOutput {
    pub device_id: String,
    pub op: String,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<ToolSnmpProbeRow>,

    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sys_descr: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sys_object_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_indicators: Option<Vec<KnowledgeIndicator>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generic_indicators: Option<Vec<KnowledgeIndicator>>,
}

impl ToolSnmpProbeOutput {
    fn new(device_id: String, op: String) -> Self {
        ToolSnmpProbeOutput { device_id, op, rows: Vec::new(), truncated: false, sys_descr: None, sys_object_id: None, vendor: None, vendor_indicators: None, generic_indicators: None }
    }
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolSnmpProbe {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    collector_snmp: CollectorSnmp,
}

#[async_trait]
impl BaseApp for ToolSnmpProbe {
    fn name(&self) -> &'static str {
        "ToolSnmpProbe"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSnmpProbe {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSnmpProbe
    }

    fn deferred(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolSnmpProbe {
    const NAME: &'static str = AiToolId::ToolSnmpProbe.as_str();

    type Args = ToolSnmpProbeArgs;
    type Output = ToolSnmpProbeOutput;
    type Error = AiToolError;

    fn description(&self) -> String {
        "对设备做一次 SNMP 探测，只读，不落任何指标。op=get 精确取值（等价 snmpget，oids 填精确实例）；op=walk 遍历子树（等价 snmpwalk，oids 只填一个根）；op=suggest 先实测这台设备的 sysObjectID/sysDescr，再返回该设备所属厂商的候选监控 OID（CPU/内存/温度等，含通用兜底候选）。suggest 只回当前这台设备厂商的切片，没有查询整个厂商库的入口，设备不可达时什么都不返回。前提：设备已在编辑页启用 SNMP 并配好团体字。标准流程：suggest 拿候选 → walk/get 逐个实测确认存在且是数字 → 才写指标。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "设备 id，取自用户的 @[device] 引用或 select_all_device 的返回，不是设备名"
                },
                "op": {
                    "type": "string",
                    "enum": ["get", "walk", "suggest"],
                    "description": "get=精确取值（可多个 OID）；walk=遍历子树（只能一个根 OID）；suggest=按设备实测的 sysObjectID 返回该厂商候选监控 OID，不需要 oids"
                },
                "oids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "数字 OID，如 1.3.6.1.2.1.1.5.0；walk 时填子树根如 1.3.6.1.2.1.2.2.1.10；suggest 时不填"
                },
                "max_rows": {
                    "type": "integer",
                    "description": "walk 最多返回多少行，默认 50，上限 200；行数超出会置 truncated=true"
                }
            },
            "required": ["device_id", "op"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let device_id = args.device_id.trim().to_string();
        if device_id.is_empty() {
            return Err(AiToolError::Execution("device_id must not be empty".to_string()));
        }
        let client = self.collector_snmp.build_client(&device_id).await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;

        let output = if args.op == SNMP_OP_SUGGEST {
            Self::suggest(&client, device_id).await?
        } else {
            if args.oids.is_empty() {
                return Err(AiToolError::Execution("oids must not be empty".to_string()));
            }
            let oids = args.oids.iter().map(|raw| Oid::from_str(raw).map_err(|error| AiToolError::Execution(format!("{}", error)))).collect::<Result<Vec<Oid>, AiToolError>>()?;
            let max_rows = args.max_rows.unwrap_or(SNMP_PROBE_ROWS_DEFAULT).clamp(1, SNMP_PROBE_ROWS_MAX);

            let (varbinds, truncated) = match args.op.as_str() {
                SNMP_OP_GET => {
                    let rows = client.get(&oids).await.map_err(|error| AiToolError::Execution(format!("SNMP get failed: {}", error)))?;
                    (rows, false)
                }
                SNMP_OP_WALK => {
                    if oids.len() != 1 {
                        return Err(AiToolError::Execution(format!("walk accepts only one root OID, got {}", oids.len())));
                    }
                    let rows = client.walk(&oids[0], max_rows).await.map_err(|error| AiToolError::Execution(format!("SNMP walk failed: {}", error)))?;
                    let truncated = rows.len() >= max_rows;
                    (rows, truncated)
                }
                other => return Err(AiToolError::Execution(format!("unrecognized operation op=\"{}\"; supported: get/walk/suggest", other))),
            };

            let mut output = ToolSnmpProbeOutput::new(device_id, args.op);
            output.truncated = truncated;
            output.rows =
                varbinds.iter().map(|varbind| ToolSnmpProbeRow { oid: varbind.oid.to_string(), value_type: varbind.value.type_name().to_string(), value: varbind.value.to_string() }).collect();
            output
        };
        let body = serde_json::to_string(&output).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(output)
    }
}

impl ToolSnmpProbe {
    async fn suggest(client: &SnmpClient, device_id: String) -> Result<ToolSnmpProbeOutput, AiToolError> {
        let oids = [
            Oid::from_str(OID_SYS_OBJECT_ID).map_err(|error| AiToolError::Execution(format!("{}", error)))?,
            Oid::from_str(OID_SYS_DESCR).map_err(|error| AiToolError::Execution(format!("{}", error)))?,
        ];
        let varbinds = client.get(&oids).await.map_err(|error| AiToolError::Execution(format!("SNMP system info probe failed: {}", error)))?;
        let sys_object_id = Self::find_value(&varbinds, OID_SYS_OBJECT_ID);
        let sys_descr = Self::find_value(&varbinds, OID_SYS_DESCR);

        let enterprise = Self::resolve_enterprise(&varbinds);
        let knowledge = vendor_knowledge()?;
        let matched = enterprise.and_then(|number| knowledge.vendors.iter().find(|vendor| vendor.enterprise == number));

        let mut output = ToolSnmpProbeOutput::new(device_id, SNMP_OP_SUGGEST.to_string());
        output.sys_descr = sys_descr;
        output.sys_object_id = sys_object_id;
        output.vendor = matched.map(|vendor| vendor.name.clone());
        output.vendor_indicators = matched.map(|vendor| vendor.indicators.clone());
        output.generic_indicators = Some(knowledge.generic.clone());
        Ok(output)
    }

    fn find_value(varbinds: &[VarBind], oid: &str) -> Option<String> {
        varbinds.iter().find(|varbind| varbind.oid.to_string() == oid).filter(|varbind| !varbind.value.is_exception()).map(|varbind| varbind.value.to_string())
    }

    fn resolve_enterprise(varbinds: &[VarBind]) -> Option<u32> {
        let varbind = varbinds.iter().find(|varbind| varbind.oid.to_string() == OID_SYS_OBJECT_ID)?;
        let SnmpValue::ObjectIdentifier(oid) = &varbind.value else {
            return None;
        };
        if oid.0.len() <= ENTERPRISE_PREFIX.len() || oid.0[..ENTERPRISE_PREFIX.len()] != ENTERPRISE_PREFIX {
            return None;
        }
        Some(oid.0[ENTERPRISE_PREFIX.len()])
    }
}
