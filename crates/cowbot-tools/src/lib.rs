pub mod code_tool;
pub mod executor;
pub mod filesystem;
pub mod registry;
pub mod risk;
pub mod sandbox;

pub use code_tool::CodeExecutionTool;
pub use executor::SubprocessSandbox;
pub use filesystem::{ReadFileTool, WriteFileTool};
pub use registry::ToolRegistry;
pub use risk::{RiskAssessment, StaticRiskClassifier};
pub use sandbox::{ExecutionResult, SandboxBackend};

#[cfg(test)]
mod tests {
    use super::*;
    use cowbot_core::tool::ToolCall;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_subprocess_echo() {
        let sandbox = SubprocessSandbox::default();
        let cmd = if cfg!(windows) {
            "echo hello cowbot"
        } else {
            "echo 'hello cowbot'"
        };
        let res = sandbox.execute(cmd, None, 5).await.unwrap();
        assert!(res.is_success());
        assert!(res.stdout.contains("hello cowbot"));
    }

    #[tokio::test]
    async fn test_tool_registry_and_execution() {
        let mut registry = ToolRegistry::new();
        let sandbox = Arc::new(SubprocessSandbox::default());
        registry.register(Arc::new(CodeExecutionTool::new(sandbox)));

        let call = ToolCall {
            id: "call-1".to_string(),
            name: "execute_code".to_string(),
            arguments: json!({
                "command": if cfg!(windows) { "echo tool test" } else { "echo 'tool test'" }
            }),
        };

        let result = registry.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("tool test"));
    }
}
