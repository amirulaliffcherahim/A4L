use a4l_core::guards::can_advance;
use a4l_core::state::{Gates, StateStore, Task, TaskRole, TaskStatus, TasksJson};
use a4l_memory::{Episode, EpisodicStore};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DesktopError {
    #[error("state store error: {0}")]
    State(#[from] a4l_core::state::StateError),
    #[error("guard failure: {0}")]
    Guard(#[from] a4l_core::guards::GuardFailure),
    #[error("episodic memory error: {0}")]
    Episodic(#[from] a4l_memory::EpisodicError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid task id: {0}")]
    InvalidTaskId(String),
}

impl serde::Serialize for DesktopError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn validate_task_id(task_id: &str) -> Result<(), DesktopError> {
    if task_id.is_empty()
        || !task_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(DesktopError::InvalidTaskId(task_id.to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSummary {
    pub name: String,
    pub filename: String,
    pub content: String,
}

#[tauri::command]
pub fn get_tasks(repo_root: String) -> Result<TasksJson, DesktopError> {
    let path = PathBuf::from(&repo_root);
    let tasks_file = path.join(".a4l").join("tasks.json");
    let store = StateStore::new(tasks_file);
    let tasks = store.load()?;
    Ok(tasks)
}

#[tauri::command]
pub fn advance_task(
    repo_root: String,
    task_id: String,
    target_role: TaskRole,
    target_status: TaskStatus,
) -> Result<Task, DesktopError> {
    validate_task_id(&task_id)?;
    let path = PathBuf::from(&repo_root);
    let tasks_file = path.join(".a4l").join("tasks.json");
    let store = StateStore::new(tasks_file);

    let task = store.get_task(&task_id)?;
    can_advance(&task, target_role, target_status)?;

    let updated = store.update_task(&task_id, |t| {
        t.state.role = target_role;
        t.state.status = target_status;
        Ok(())
    })?;

    Ok(updated)
}

#[tauri::command]
pub fn approve_hitl(repo_root: String, task_id: String) -> Result<Task, DesktopError> {
    validate_task_id(&task_id)?;
    let path = PathBuf::from(&repo_root);
    let tasks_file = path.join(".a4l").join("tasks.json");
    let store = StateStore::new(tasks_file);

    let task = store.get_task(&task_id)?;
    can_advance(&task, TaskRole::Human, TaskStatus::Completed)?;

    let updated = store.update_task(&task_id, |t| {
        t.state.status = TaskStatus::Completed;
        Ok(())
    })?;

    Ok(updated)
}

#[tauri::command]
pub fn send_back_hitl(
    repo_root: String,
    task_id: String,
    notes: String,
) -> Result<Task, DesktopError> {
    validate_task_id(&task_id)?;
    let path = PathBuf::from(&repo_root);
    let tasks_file = path.join(".a4l").join("tasks.json");
    let store = StateStore::new(tasks_file);

    let task = store.get_task(&task_id)?;
    can_advance(&task, TaskRole::Planner, TaskStatus::SentBack)?;

    // Append notes to context bundle
    if let Some(bundle_rel) = &task.state.context_bundle {
        let bundle_path = path.join(bundle_rel);
        if bundle_path.exists() {
            let mut content = std::fs::read_to_string(&bundle_path).unwrap_or_default();
            content.push_str(&format!("\n\n## HITL Reviewer Notes\n{}", notes.trim()));
            std::fs::write(&bundle_path, content)?;
        }
    }

    let updated = store.update_task(&task_id, |t| {
        t.state.role = TaskRole::Planner;
        t.state.status = TaskStatus::SentBack;
        t.state.attempt += 1;
        Ok(())
    })?;

    Ok(updated)
}

#[tauri::command]
pub fn get_diff(repo_root: String, _task_id: String) -> Result<String, DesktopError> {
    let path = PathBuf::from(&repo_root);
    let output = Command::new("git")
        .current_dir(&path)
        .args(["diff", "HEAD"])
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
        _ => Ok("No active git diff detected.".to_string()),
    }
}

#[tauri::command]
pub fn get_gate_proofs(repo_root: String, task_id: String) -> Result<Gates, DesktopError> {
    validate_task_id(&task_id)?;
    let path = PathBuf::from(&repo_root);
    let tasks_file = path.join(".a4l").join("tasks.json");
    let store = StateStore::new(tasks_file);
    let task = store.get_task(&task_id)?;
    Ok(task.gates)
}

#[tauri::command]
pub fn list_skills(repo_root: String) -> Result<Vec<SkillSummary>, DesktopError> {
    let path = PathBuf::from(&repo_root);
    let skills_dir = path.join(".a4l").join("skills");
    if !skills_dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.extension().and_then(|s| s.to_str()) == Some("md") {
            let filename = entry_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let name = entry_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let content = std::fs::read_to_string(&entry_path)?;
            summaries.push(SkillSummary {
                name,
                filename,
                content,
            });
        }
    }

    Ok(summaries)
}

#[tauri::command]
pub fn search_memory(repo_root: String, query: String) -> Result<Vec<Episode>, DesktopError> {
    let path = PathBuf::from(&repo_root);
    let db_path = path.join(".a4l").join("memory.db");
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    let store = EpisodicStore::open(&db_path)?;
    let results = store.search_by_symptom(&query)?;
    Ok(results)
}
