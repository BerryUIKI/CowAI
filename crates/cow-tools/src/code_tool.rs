use crate::sandbox::SandboxBackend;
use async_trait::async_trait;
use cow_core::error::CowError;
use cow_core::tool::{Tool, ToolCall, ToolResult};
use serde_json::json;
use std::sync::Arc;

/// Tool exposing code/command execution to LLM agents.
pub struct CodeExecutionTool {
    backend: Arc<dyn SandboxBackend>,
}

impl CodeExecutionTool {
    pub fn new(backend: Arc<dyn SandboxBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl Tool for CodeExecutionTool {
    fn name(&self) -> &str {
        "execute_code"
    }

    fn description(&self) -> &str {
        "Executes a shell command or script in the sandbox environment and returns stdout/stderr."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command or script line to execute"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Optional timeout in seconds (default: 30)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, CowError> {
        let command = call
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CowError::ToolExecution {
                name: self.name().to_string(),
                message: "Missing 'command' argument".to_string(),
            })?;

        let timeout_secs = call
            .arguments
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let result = self.backend.execute(command, None, timeout_secs).await?;

        if result.is_success() {
            Ok(ToolResult::success(&call.id, result.formatted_output()))
        } else {
            Ok(ToolResult::failure(&call.id, result.formatted_output()))
        }
    }
}
