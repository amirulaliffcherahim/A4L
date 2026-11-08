use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

use crate::state::Task;

#[derive(Debug, Error)]
pub enum HandoffError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub what: String,
    pub why: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandoffNote {
    pub goal: String,
    #[serde(default)]
    pub decisions: Vec<Decision>,
    pub next: String,
    #[serde(default)]
    pub pitfalls: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffFrom {
    pub role: String,
    pub mode: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRefs {
    #[serde(default)]
    pub diff: String,
    #[serde(default)]
    pub gate_logs: Vec<String>,
    #[serde(default)]
    pub failure_bundle: Option<String>,
    #[serde(default)]
    pub history: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffFacts {
    pub status: String,
    pub todos: Vec<crate::state::TodoItem>,
    pub gates: crate::state::Gates,
    pub diff_stat: HashMap<String, String>,
    pub breaker: crate::state::CircuitBreaker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffPacket {
    pub v: u32,
    pub task_id: String,
    pub seq: u64,
    pub created_at: DateTime<Utc>,
    pub trigger: String, // "todo_verified" | "role_transition" | "mode_switch" | "model_switch" | "session_end" | "breaker_trip" | "manual"
    pub from: HandoffFrom,
    pub facts: HandoffFacts,
    pub note: HandoffNote,
    pub note_stale: bool,
    pub refs: HandoffRefs,
}

impl HandoffPacket {
    pub fn new_from_task(
        task: &Task,
        seq: u64,
        trigger: &str,
        model: &str,
        note: HandoffNote,
        note_stale: bool,
    ) -> Self {
        let facts = HandoffFacts {
            status: format!("{:?}", task.state.status).to_lowercase(),
            todos: task.todos.clone(),
            gates: task.gates.clone(),
            diff_stat: HashMap::new(),
            breaker: task.circuit_breaker.clone(),
        };

        let mut gate_logs = Vec::new();
        if let Some(log) = &task.gates.l1.log {
            gate_logs.push(log.clone());
        }
        if let Some(log) = &task.gates.l2.log {
            gate_logs.push(log.clone());
        }
        if let Some(log) = &task.gates.l3.log {
            gate_logs.push(log.clone());
        }
        if let Some(log) = &task.gates.l4.log {
            gate_logs.push(log.clone());
        }

        let refs = HandoffRefs {
            diff: String::new(),
            gate_logs,
            failure_bundle: task
                .circuit_breaker
                .reason
                .map(|_| format!(".a4l/failures/{}.json", task.id)),
            history: format!(".a4l/history/{}", task.id),
        };

        Self {
            v: 1,
            task_id: task.id.clone(),
            seq,
            created_at: Utc::now(),
            trigger: trigger.to_string(),
            from: HandoffFrom {
                role: format!("{:?}", task.state.role).to_lowercase(),
                mode: "standard".to_string(),
                model: model.to_string(),
            },
            facts,
            note,
            note_stale,
            refs,
        }
    }

    /// Truncate note by priority (next, pitfalls, decisions, goal) to stay within token cap
    pub fn enforce_note_cap(&mut self, max_tokens: usize) {
        let mut estimated_tokens =
            (serde_json::to_string(&self.note).unwrap_or_default().len() as f64 / 4.0) as usize;
        if estimated_tokens <= max_tokens {
            return;
        }

        // 1. Truncate next
        if estimated_tokens > max_tokens && self.note.next.len() > 100 {
            self.note.next.truncate(100);
            estimated_tokens =
                (serde_json::to_string(&self.note).unwrap_or_default().len() as f64 / 4.0) as usize;
        }

        // 2. Truncate pitfalls
        while estimated_tokens > max_tokens && !self.note.pitfalls.is_empty() {
            self.note.pitfalls.pop();
            estimated_tokens =
                (serde_json::to_string(&self.note).unwrap_or_default().len() as f64 / 4.0) as usize;
        }

        // 3. Truncate decisions
        while estimated_tokens > max_tokens && !self.note.decisions.is_empty() {
            self.note.decisions.pop();
            estimated_tokens =
                (serde_json::to_string(&self.note).unwrap_or_default().len() as f64 / 4.0) as usize;
        }

        // 4. Truncate goal
        if estimated_tokens > max_tokens && self.note.goal.len() > 100 {
            self.note.goal.truncate(100);
        }
    }

    pub fn save(&self, repo_root: &Path) -> Result<(), HandoffError> {
        let handoffs_dir = repo_root.join(".a4l").join("handoffs");
        std::fs::create_dir_all(&handoffs_dir)?;

        let filename = format!("{}.latest.json", self.task_id);
        let path = handoffs_dir.join(filename);
        let json_data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json_data)?;
        Ok(())
    }

    pub fn load_latest(repo_root: &Path, task_id: &str) -> Result<Option<Self>, HandoffError> {
        let path = repo_root
            .join(".a4l")
            .join("handoffs")
            .join(format!("{}.latest.json", task_id));

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)?;
        let packet: HandoffPacket = serde_json::from_str(&content)?;
        Ok(Some(packet))
    }
}
