use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

use crate::state::{CircuitBreakerStatus, Task, TaskRole, TaskStatus, TripReason};

#[derive(Debug, Error)]
pub enum BreakerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git command error: {0}")]
    Git(String),
    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureBundle {
    pub task_id: String,
    pub attempt: u32,
    pub trip_reason: TripReason,
    pub error_hashes: Vec<String>,
    pub last_gate_logs: Vec<String>,
    pub quarantine_branch: String,
    pub created_at: chrono::DateTime<Utc>,
}

pub struct CircuitBreakerController {
    repo_root: PathBuf,
    max_attempts: u32,
}

impl CircuitBreakerController {
    pub fn new(repo_root: impl AsRef<Path>, max_attempts: u32) -> Self {
        Self {
            repo_root: repo_root.as_ref().to_path_buf(),
            max_attempts,
        }
    }

    /// Check if task should trip the breaker:
    /// 1. Any error hash with count >= 2
    /// 2. Current attempt >= max_attempts and Maker gates failed
    pub fn check_trip(&self, task: &Task, maker_failed: bool) -> Option<TripReason> {
        // 1. Duplicate error hash >= 2
        for err in &task.errors {
            if err.count >= 2 {
                return Some(TripReason::DuplicateError);
            }
        }

        // 2. Max attempts reached
        if task.state.attempt >= self.max_attempts && maker_failed {
            return Some(TripReason::MaxAttempts);
        }

        None
    }

    /// Execute the Planner Rebase on trip:
    /// - Stashes dirty state / creates quarantine branch wip/<task-id>-attempt-<n>
    /// - Resets git tree to clean commit
    /// - Purges old context bundle
    /// - Writes failure bundle to .a4l/failures/<task-id>.json
    /// - Updates task state back to Planner with attempt + 1
    pub fn trip_and_rebase(
        &self,
        task: &mut Task,
        reason: TripReason,
    ) -> Result<FailureBundle, BreakerError> {
        let branch_name = format!("wip/{}-attempt-{}", task.id, task.state.attempt);

        // 1. Create quarantine branch if in git repo
        let _ = Command::new("git")
            .current_dir(&self.repo_root)
            .args(["checkout", "-B", &branch_name])
            .output();
        let _ = Command::new("git")
            .current_dir(&self.repo_root)
            .args(["add", "-A"])
            .output();
        let _ = Command::new("git")
            .current_dir(&self.repo_root)
            .args([
                "commit",
                "-m",
                &format!("quarantine: {} trip {:?}", task.id, reason),
            ])
            .output();

        // 2. Reset tree to base commit or clean state
        let _ = Command::new("git")
            .current_dir(&self.repo_root)
            .args(["checkout", "main"])
            .output();

        // 3. Purge context bundle file if exists
        if let Some(bundle_rel) = &task.state.context_bundle {
            let bundle_path = self.repo_root.join(bundle_rel);
            if bundle_path.exists() {
                let _ = std::fs::remove_file(bundle_path);
            }
        }

        // 4. Collect logs and failure bundle
        let failures_dir = self.repo_root.join(".a4l").join("failures");
        std::fs::create_dir_all(&failures_dir)?;

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

        let bundle = FailureBundle {
            task_id: task.id.clone(),
            attempt: task.state.attempt,
            trip_reason: reason,
            error_hashes: task.errors.iter().map(|e| e.hash.clone()).collect(),
            last_gate_logs: gate_logs,
            quarantine_branch: branch_name.clone(),
            created_at: Utc::now(),
        };

        let bundle_file = failures_dir.join(format!("{}.json", task.id));
        let bundle_json = serde_json::to_string_pretty(&bundle)?;
        std::fs::write(bundle_file, bundle_json)?;

        // 5. Update task state
        task.circuit_breaker.status = CircuitBreakerStatus::Tripped;
        task.circuit_breaker.reason = Some(reason);
        task.circuit_breaker.quarantine_branch = Some(branch_name);

        // Rebase for next attempt
        task.state.role = TaskRole::Planner;
        task.state.status = TaskStatus::Planned;
        task.state.attempt += 1;
        task.state.context_bundle = None;

        Ok(bundle)
    }
}
