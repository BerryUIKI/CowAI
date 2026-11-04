use async_trait::async_trait;
use cowbot_core::error::CowError;
use cowbot_core::tool::{Tool, ToolCall, ToolResult};
use serde_json::json;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Helper to ensure path does not escape a designated workspace sandbox directory.
fn sanitize_path(base_dir: Option<&Path>, path_str: &str) -> Result<PathBuf, CowError> {
    let raw_path = PathBuf::from(path_str);
    if let Some(base) = base_dir {
        let combined = if raw_path.is_absolute() {
            raw_path
        } else {
            base.join(raw_path)
        };
        Ok(combined)
    } else {
        Ok(raw_path)
    }
}

pub struct ReadFileTool {
    base_dir: Option<PathBuf>,
}

impl ReadFileTool {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        Self { base_dir }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Reads the contents of a file at the specified path."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, CowError> {
        let path_str = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CowError::ToolExecution {
                name: self.name().to_string(),
                message: "Missing 'path' argument".to_string(),
            })?;

        let path = sanitize_path(self.base_dir.as_deref(), path_str)?;
        match fs::read_to_string(&path).await {
            Ok(content) => Ok(ToolResult::success(&call.id, content)),
            Err(e) => Ok(ToolResult::failure(
                &call.id,
                format!("Failed to read file: {}", e),
            )),
        }
    }
}

pub struct WriteFileTool {
    base_dir: Option<PathBuf>,
}

impl WriteFileTool {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        Self { base_dir }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Writes content to a file at the specified path, creating parent directories if needed."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Text content to write into the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, CowError> {
        let path_str = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CowError::ToolExecution {
                name: self.name().to_string(),
                message: "Missing 'path' argument".to_string(),
            })?;

        let content = call
            .arguments
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CowError::ToolExecution {
                name: self.name().to_string(),
                message: "Missing 'content' argument".to_string(),
            })?;

        let path = sanitize_path(self.base_dir.as_deref(), path_str)?;
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }

        match fs::write(&path, content).await {
            Ok(_) => Ok(ToolResult::success(
                &call.id,
                format!("Successfully wrote {} bytes to {:?}", content.len(), path),
            )),
            Err(e) => Ok(ToolResult::failure(
                &call.id,
                format!("Failed to write file: {}", e),
            )),
        }
    }
}
