use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerTokens {
    pub layer1_kernel: u32,
    pub layer2_tools: u32,
    pub layer3_profile: u32,
    pub layer4_role_doc: u32,
    pub layer5_state: u32,
    pub layer6_handoff: u32,
    pub layer7_tail: u32,
    pub layer8_user_turn: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub timestamp: DateTime<Utc>,
    pub task_id: Option<String>,
    pub model: String,
    pub layers: LayerTokens,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cached_prompt_tokens: u32,
    pub is_estimated: bool,
    pub trimmed_bytes: usize,
}

pub struct TokenLedger {
    file_path: PathBuf,
}

impl TokenLedger {
    pub fn new(repo_root: impl AsRef<Path>) -> Self {
        let history_dir = repo_root.as_ref().join(".a4l").join("history");
        let file_path = history_dir.join("ledger.jsonl");
        Self { file_path }
    }

    pub fn record(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;

        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }
}
