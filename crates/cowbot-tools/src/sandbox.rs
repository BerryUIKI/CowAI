use async_trait::async_trait;
use cowbot_core::error::CowResult;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of executing a command or code snippet in a sandbox environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
    pub timed_out: bool,
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        !self.timed_out && self.exit_code == Some(0)
    }

    pub fn formatted_output(&self) -> String {
        let mut out = String::new();
        if self.timed_out {
            out.push_str("[TIMEOUT] Execution timed out.\n");
        }
        if !self.stdout.is_empty() {
            out.push_str(&self.stdout);
        }
        if !self.stderr.is_empty() {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("[STDERR]\n");
            out.push_str(&self.stderr);
        }
        out
    }
}

/// Pluggable sandbox backend trait (e.g. Subprocess, Docker, Wasm).
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    async fn execute(
        &self,
        command: &str,
        cwd: Option<&Path>,
        timeout_secs: u64,
    ) -> CowResult<ExecutionResult>;
}
