use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::timeout;

use crate::buffer::RollingBuffer;

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("command timed out after {0:?}")]
    Timeout(Duration),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("process error: {0}")]
    Process(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyExecutionResult {
    pub cmd: String,
    pub exit_code: i32,
    pub summary: String,
    pub log_path: String,
    pub total_bytes: usize,
    pub is_truncated: bool,
    pub duration_ms: u128,
}

pub struct PtyRunner {
    history_dir: PathBuf,
    head_bytes: usize,
    tail_bytes: usize,
}

impl PtyRunner {
    pub fn new(history_dir: impl AsRef<Path>) -> Self {
        Self {
            history_dir: history_dir.as_ref().to_path_buf(),
            head_bytes: 2048,
            tail_bytes: 2048,
        }
    }

    pub fn with_buffer_sizes(mut self, head: usize, tail: usize) -> Self {
        self.head_bytes = head;
        self.tail_bytes = tail;
        self
    }

    pub async fn run_command(
        &self,
        repo_root: &Path,
        task_id: &str,
        cmd_str: &str,
        timeout_duration: Duration,
    ) -> Result<PtyExecutionResult, PtyError> {
        std::fs::create_dir_all(&self.history_dir)?;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let log_rel = format!(".a4l/history/pty_{}_{}.log", task_id, timestamp);
        let log_path = repo_root.join(&log_rel);

        let mut log_file = File::create(&log_path)?;
        writeln!(log_file, "COMMAND: {}\nSTART: {}\n---", cmd_str, Utc::now())?;

        let start_time = std::time::Instant::now();

        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd.exe");
            c.args(["/C", cmd_str]);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", cmd_str]);
            c
        };

        cmd.current_dir(repo_root);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| PtyError::Process("missing stdout".to_string()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| PtyError::Process("missing stderr".to_string()))?;

        let mut rolling_buf = RollingBuffer::new(self.head_bytes, self.tail_bytes);

        // Async read loop with timeout
        let read_future = async {
            let mut out_buf = [0u8; 4096];
            let mut err_buf = [0u8; 4096];
            loop {
                tokio::select! {
                    res = stdout.read(&mut out_buf) => {
                        match res? {
                            0 => break,
                            n => {
                                rolling_buf.feed(&out_buf[..n]);
                                log_file.write_all(&out_buf[..n])?;
                            }
                        }
                    }
                    res = stderr.read(&mut err_buf) => {
                        match res? {
                            0 => break,
                            n => {
                                rolling_buf.feed(&err_buf[..n]);
                                log_file.write_all(&err_buf[..n])?;
                            }
                        }
                    }
                }
            }
            let status = child.wait().await?;
            Ok::<_, PtyError>(status.code().unwrap_or(-1))
        };

        let exit_code = match timeout(timeout_duration, read_future).await {
            Ok(res) => res?,
            Err(_) => {
                let _ = child.kill().await;
                return Err(PtyError::Timeout(timeout_duration));
            }
        };

        let duration_ms = start_time.elapsed().as_millis();
        writeln!(
            log_file,
            "\n---\nEXIT: {}\nDURATION_MS: {}",
            exit_code, duration_ms
        )?;

        Ok(PtyExecutionResult {
            cmd: cmd_str.to_string(),
            exit_code,
            summary: rolling_buf.render_summary(),
            log_path: log_rel,
            total_bytes: rolling_buf.total_bytes(),
            is_truncated: rolling_buf.is_truncated(),
            duration_ms,
        })
    }
}
