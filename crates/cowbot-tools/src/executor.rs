use crate::sandbox::{ExecutionResult, SandboxBackend};
use async_trait::async_trait;
use cowbot_core::error::{CowError, CowResult};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

/// Subprocess-based sandbox execution runner.
#[derive(Debug, Clone)]
pub struct SubprocessSandbox {
    base_dir: Option<PathBuf>,
    max_output_bytes: usize,
}

impl SubprocessSandbox {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        Self {
            base_dir,
            max_output_bytes: 1024 * 1024, // 1MB output limit
        }
    }
}

impl Default for SubprocessSandbox {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl SandboxBackend for SubprocessSandbox {
    async fn execute(
        &self,
        command: &str,
        cwd: Option<&Path>,
        timeout_secs: u64,
    ) -> CowResult<ExecutionResult> {
        let start = Instant::now();
        let target_dir = cwd.or(self.base_dir.as_deref());

        debug!(
            cmd = command,
            ?target_dir,
            timeout_secs,
            "Spawning subprocess"
        );

        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        if let Some(dir) = target_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| CowError::ToolExecution {
            name: "subprocess".to_string(),
            message: format!("Failed to spawn process: {}", e),
        })?;

        let duration = Duration::from_secs(timeout_secs);
        let res = timeout(duration, child.wait_with_output()).await;

        let elapsed = start.elapsed().as_millis();

        match res {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                Ok(ExecutionResult {
                    exit_code: output.status.code(),
                    stdout: truncate_output(&stdout, self.max_output_bytes),
                    stderr: truncate_output(&stderr, self.max_output_bytes),
                    duration_ms: elapsed,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => Err(CowError::ToolExecution {
                name: "subprocess".to_string(),
                message: format!("Process execution failed: {}", e),
            }),
            Err(_) => {
                warn!(
                    cmd = command,
                    "Execution timed out, killed child process via kill_on_drop"
                );
                Ok(ExecutionResult {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Execution timed out".to_string(),
                    duration_ms: elapsed,
                    timed_out: true,
                })
            }
        }
    }
}

fn truncate_output(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}... [Output truncated]", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::truncate_output;

    #[test]
    fn truncation_keeps_short_output() {
        assert_eq!(truncate_output("hello", 10), "hello");
    }

    #[test]
    fn truncation_respects_utf8_boundaries() {
        // "你好" is six bytes; a five-byte limit must not split the second character.
        let truncated = truncate_output("你好", 5);

        assert_eq!(truncated, "你... [Output truncated]");
    }
}
