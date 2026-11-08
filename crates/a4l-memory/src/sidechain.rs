use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SidechainError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json serialize error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidechainEntry {
    pub timestamp: DateTime<Utc>,
    pub subtask_id: Option<String>,
    pub tool_name: String,
    pub payload: serde_json::Value,
}

pub struct SidechainLogger {
    history_dir: PathBuf,
}

impl SidechainLogger {
    pub fn new(history_dir: impl AsRef<Path>) -> Self {
        Self {
            history_dir: history_dir.as_ref().to_path_buf(),
        }
    }

    pub fn log_event(
        &self,
        task_id: &str,
        subtask_id: Option<&str>,
        tool_name: &str,
        payload: serde_json::Value,
    ) -> Result<String, SidechainError> {
        std::fs::create_dir_all(&self.history_dir)?;
        let log_rel = format!("{}.sidechain.jsonl", task_id);
        let log_path = self.history_dir.join(&log_rel);

        let entry = SidechainEntry {
            timestamp: Utc::now(),
            subtask_id: subtask_id.map(|s| s.to_string()),
            tool_name: tool_name.to_string(),
            payload,
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let line = serde_json::to_string(&entry)?;
        writeln!(file, "{}", line)?;

        Ok(log_rel)
    }
}
