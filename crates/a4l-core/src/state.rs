use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("task not found: {0}")]
    TaskNotFound(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Planned,
    InProgress,
    Review,
    Verifying,
    AwaitingHitl,
    Completed,
    Failed,
    SentBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRole {
    Planner,
    Maker,
    Checker,
    Human,
}

impl std::fmt::Display for TaskRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskRole::Planner => write!(f, "planner"),
            TaskRole::Maker => write!(f, "maker"),
            TaskRole::Checker => write!(f, "checker"),
            TaskRole::Human => write!(f, "human"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    Done,
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub status: TodoStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriteria {
    pub id: String,
    pub check: String,
    #[serde(default = "default_unverified")]
    pub status: String, // "unverified" | "verified"
}

fn default_unverified() -> String {
    "unverified".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateResult {
    pub cmd: String,
    #[serde(default)]
    pub exit: Option<i32>,
    #[serde(default)]
    pub log: Option<String>,
    #[serde(default)]
    pub ran_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Gates {
    #[serde(default)]
    pub l1: GateResult,
    #[serde(default)]
    pub l2: GateResult,
    #[serde(default)]
    pub l3: GateResult,
    #[serde(default)]
    pub l4: GateResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub hash: String,
    pub count: u32,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitBreakerStatus {
    Armed,
    Tripped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TripReason {
    DuplicateError,
    DiffOscillation,
    MaxAttempts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub status: CircuitBreakerStatus,
    #[serde(default)]
    pub reason: Option<TripReason>,
    #[serde(default)]
    pub quarantine_branch: Option<String>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            status: CircuitBreakerStatus::Armed,
            reason: None,
            quarantine_branch: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub status: TaskStatus,
    pub role: TaskRole,
    #[serde(default = "default_attempt")]
    pub attempt: u32,
    #[serde(default)]
    pub context_bundle: Option<String>,
}

fn default_attempt() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub state: TaskState,
    #[serde(default)]
    pub todos: Vec<TodoItem>,
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriteria>,
    #[serde(default)]
    pub gates: Gates,
    #[serde(default)]
    pub errors: Vec<ErrorRecord>,
    #[serde(default)]
    pub circuit_breaker: CircuitBreaker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksJson {
    pub version: u32,
    pub queue: Vec<Task>,
}

impl Default for TasksJson {
    fn default() -> Self {
        Self {
            version: 1,
            queue: Vec::new(),
        }
    }
}

pub struct StateStore {
    path: std::path::PathBuf,
}

impl StateStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> Result<TasksJson, StateError> {
        if !self.path.exists() {
            return Ok(TasksJson::default());
        }

        let mut file = File::open(&self.path)?;
        file.lock_shared()?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;
        file.unlock()?;

        if content.trim().is_empty() {
            return Ok(TasksJson::default());
        }

        let tasks: TasksJson = serde_json::from_str(&content)?;
        Ok(tasks)
    }

    pub fn save(&self, tasks: &TasksJson) -> Result<(), StateError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let lock_path = self.path.with_extension("lock");
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let temp_path = self.path.with_extension("tmp");
        let content = serde_json::to_string_pretty(tasks)?;

        {
            let mut temp_file = File::create(&temp_path)?;
            temp_file.write_all(content.as_bytes())?;
            temp_file.sync_all()?;
        }

        std::fs::rename(&temp_path, &self.path)?;
        lock_file.unlock()?;
        let _ = std::fs::remove_file(lock_path);

        Ok(())
    }

    pub fn get_task(&self, task_id: &str) -> Result<Task, StateError> {
        let tasks = self.load()?;
        tasks
            .queue
            .into_iter()
            .find(|t| t.id == task_id)
            .ok_or_else(|| StateError::TaskNotFound(task_id.to_string()))
    }

    pub fn update_task<F>(&self, task_id: &str, update_fn: F) -> Result<Task, StateError>
    where
        F: FnOnce(&mut Task) -> Result<(), StateError>,
    {
        let lock_path = self.path.with_extension("lock");
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let mut tasks = if self.path.exists() {
            let mut file = File::open(&self.path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            if content.trim().is_empty() {
                TasksJson::default()
            } else {
                serde_json::from_str(&content)?
            }
        } else {
            TasksJson::default()
        };

        let task = tasks
            .queue
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| {
                let _ = lock_file.unlock();
                StateError::TaskNotFound(task_id.to_string())
            })?;

        update_fn(task)?;
        task.updated_at = Utc::now();
        let cloned_task = task.clone();

        let temp_path = self.path.with_extension("tmp");
        let content = serde_json::to_string_pretty(&tasks)?;
        {
            let mut temp_file = File::create(&temp_path)?;
            temp_file.write_all(content.as_bytes())?;
            temp_file.sync_all()?;
        }
        std::fs::rename(&temp_path, &self.path)?;

        let _ = lock_file.unlock();
        let _ = std::fs::remove_file(lock_path);

        Ok(cloned_task)
    }
}
