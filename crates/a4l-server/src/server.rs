use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::approvals::{ApprovalDecision, ApprovalError, ApprovalManager, PendingApproval};
use crate::events::ServerEvent;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("unauthorized: invalid or missing auth token")]
    Unauthorized,
    #[error("approval error: {0}")]
    Approval(#[from] ApprovalError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String, // default "127.0.0.1:4040"
    pub auth_token: String,
    pub allow_remote_merge: bool,
    pub approval_timeout_secs: i64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:4040".to_string(),
            auth_token: "secret-a4l-token".to_string(),
            allow_remote_merge: false,
            approval_timeout_secs: 300,
        }
    }
}

pub struct DaemonServer {
    config: ServerConfig,
    approval_mgr: Arc<ApprovalManager>,
}

impl DaemonServer {
    pub fn new(config: ServerConfig) -> Self {
        let approval_mgr = Arc::new(ApprovalManager::new(
            config.approval_timeout_secs,
            config.allow_remote_merge,
        ));
        Self {
            config,
            approval_mgr,
        }
    }

    pub fn verify_token(&self, token_header: Option<&str>) -> Result<(), ServerError> {
        let expected = &self.config.auth_token;
        match token_header {
            Some(t) if t == expected || t == format!("Bearer {}", expected) => Ok(()),
            _ => Err(ServerError::Unauthorized),
        }
    }

    pub fn approval_manager(&self) -> Arc<ApprovalManager> {
        self.approval_mgr.clone()
    }

    pub fn request_approval(
        &self,
        task_id: &str,
        action: &str,
        is_merge: bool,
    ) -> (PendingApproval, ServerEvent) {
        let approval = self.approval_mgr.create_request(task_id, action, is_merge);
        let event = ServerEvent::ApprovalRequested {
            request_id: approval.request_id.clone(),
            task_id: approval.task_id.clone(),
            action: approval.action.clone(),
            expires_at: approval.expires_at,
        };
        (approval, event)
    }

    pub fn submit_approval(
        &self,
        token: Option<&str>,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> Result<(), ServerError> {
        self.verify_token(token)?;
        self.approval_mgr.resolve(request_id, decision)?;
        Ok(())
    }
}
