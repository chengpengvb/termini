use crate::module::ai::tools::common::document::script_runtime::ScriptRuntime;

const BRIDGE_PRELUDE_JS: &str = include_str!("../../../../../../resource/script/bridge_prelude.js");
const BRIDGE_PRELUDE_PY: &str = include_str!("../../../../../../resource/script/bridge_prelude.py");

pub struct BridgePrelude;

impl BridgePrelude {
    pub fn wrap(runtime: ScriptRuntime, code: &str) -> String {
        let prelude = match runtime {
            ScriptRuntime::Deno | ScriptRuntime::Node | ScriptRuntime::Js => BRIDGE_PRELUDE_JS,
            ScriptRuntime::Python => BRIDGE_PRELUDE_PY,
        };
        format!("{}\n{}", prelude, code)
    }
}
