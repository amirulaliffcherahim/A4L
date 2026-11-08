use chrono::Utc;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

use crate::state::GateResult;

#[derive(Debug, Error)]
pub enum GateError {
    #[error("command execution error: {0}")]
    Io(#[from] std::io::Error),
    #[error("command string is empty")]
    EmptyCommand,
}

pub struct GateRunner {
    repo_root: PathBuf,
    history_dir: PathBuf,
}

impl GateRunner {
    pub fn new(repo_root: impl AsRef<Path>) -> Self {
        let repo_root = repo_root.as_ref().to_path_buf();
        let history_dir = repo_root.join(".a4l").join("history");
        Self {
            repo_root,
            history_dir,
        }
    }

    pub fn run_gate(
        &self,
        task_id: &str,
        tier_name: &str,
        cmd_str: &str,
    ) -> Result<GateResult, GateError> {
        let trimmed = cmd_str.trim();
        if trimmed.is_empty() {
            return Err(GateError::EmptyCommand);
        }

        std::fs::create_dir_all(&self.history_dir)?;
        let log_rel = format!(".a4l/history/{}.{}.log", task_id, tier_name.to_lowercase());
        let log_path = self.repo_root.join(&log_rel);

        let now = Utc::now();

        // Run in shell
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd.exe");
            c.args(["/C", trimmed]);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", trimmed]);
            c
        };

        cmd.current_dir(&self.repo_root);

        let output = cmd.output()?;
        let exit_code = output.status.code().unwrap_or(-1);

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let log_content = format!(
            "--- GATE RUN: {} [{}] ---\nCOMMAND: {}\nEXIT: {}\nTIME: {}\n\n=== STDOUT ===\n{}\n\n=== STDERR ===\n{}\n",
            tier_name, task_id, trimmed, exit_code, now, stdout, stderr
        );

        std::fs::write(&log_path, log_content)?;

        Ok(GateResult {
            cmd: trimmed.to_string(),
            exit: Some(exit_code),
            log: Some(log_rel),
            ran_at: Some(now),
        })
    }
}
