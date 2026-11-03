use cow_core::error::{CowError, CowResult};
use cow_core::tool::{Tool, ToolCall, ToolDefinition, ToolResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry maintaining all tools available in the platform or assigned to agents.
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    pub fn definitions_for(&self, names: &[String]) -> Vec<ToolDefinition> {
        names
            .iter()
            .filter_map(|n| self.tools.get(n).map(|t| t.definition()))
            .collect()
    }

    pub async fn execute(&self, call: &ToolCall) -> CowResult<ToolResult> {
        let tool = self
            .get(&call.name)
            .ok_or_else(|| CowError::ToolNotFound(call.name.clone()))?;
        tool.execute(call).await
    }
}
