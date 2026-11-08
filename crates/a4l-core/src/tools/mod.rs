pub mod ask_user;
pub mod bash;
pub mod edit;
pub mod read;
pub mod todo;
pub mod write;

use a4l_policy::{ActionRequest, PolicyDecision, PolicyEngine};
use std::path::Path;
use thiserror::Error;

use crate::provider::ToolDefinition;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("policy confirmation required: {0}")]
    PolicyAskRequired(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("tool error: {0}")]
    Execution(String),
}

pub struct ToolContext<'a> {
    pub repo_root: &'a Path,
    pub active_task_id: Option<String>,
    pub policy_engine: &'a PolicyEngine,
    pub is_headless: bool,
}

pub fn get_builtin_tools() -> Vec<ToolDefinition> {
    vec![
        read::definition(),
        write::definition(),
        edit::definition(),
        bash::definition(),
        todo::definition(),
        ask_user::definition(),
    ]
}

pub fn check_policy(engine: &PolicyEngine, req: &ActionRequest) -> Result<(), ToolError> {
    match engine.evaluate(req) {
        PolicyDecision::Allow => Ok(()),
        PolicyDecision::Ask(reason) => Err(ToolError::PolicyAskRequired(reason)),
        PolicyDecision::Deny(reason) => Err(ToolError::PolicyDenied(reason)),
    }
}
