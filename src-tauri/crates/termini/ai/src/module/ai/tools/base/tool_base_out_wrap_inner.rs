use serde_json::Value;
use rig::tool::{DynamicTool, Tool, ToolSet};
use crate::module::ai::tools::tool_base::ToolBase;

pub trait ToolBaseOutWrapInner: Clone + Send + Sync + 'static {
    fn persist(&self) -> bool;
    fn into_set(self) -> ToolSet;
    fn tool_name(&self) -> String;
    fn tool_parameters(&self) -> Value;
    fn tool_description(&self) -> String;
}

impl<T> ToolBaseOutWrapInner for T
where
    T: Tool + ToolBase + Clone + 'static,
{
    fn persist(&self) -> bool {
        ToolBase::persist(self)
    }

    fn into_set(self) -> ToolSet {
        ToolSet::from_tools(vec![self])
    }

    fn tool_name(&self) -> String {
        T::NAME.to_string()
    }

    fn tool_parameters(&self) -> Value {
        Tool::parameters(self)
    }

    fn tool_description(&self) -> String {
        Tool::description(self)
    }
}

impl ToolBaseOutWrapInner for DynamicTool {
    fn persist(&self) -> bool {
        false
    }

    fn into_set(self) -> ToolSet {
        ToolSet::from_dynamic_tools(vec![self])
    }

    fn tool_name(&self) -> String {
        self.name().to_string()
    }

    fn tool_parameters(&self) -> Value {
        self.definition().parameters
    }

    fn tool_description(&self) -> String {
        self.definition().description
    }
}
