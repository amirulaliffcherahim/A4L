use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ApprovalError {
    #[error("request '{0}' not found")]
    NotFound(String),
    #[error("request '{0}' has expired (timeout = deny)")]
    Expired(String),
    #[error("remote merge is disabled by policy (allow_remote_merge is false)")]
    RemoteMergeDisabled,
    #[error("request '{0}' already resolved")]
    AlreadyResolved(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub request_id: String,
    pub task_id: String,
    pub action: String,
    pub is_merge_action: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub decision: Option<ApprovalDecision>,
}

pub struct ApprovalManager {
    timeout_duration: Duration,
    allow_remote_merge: bool,
    requests: Mutex<HashMap<String, PendingApproval>>,
    counter: Mutex<u64>,
}

impl ApprovalManager {
    pub fn new(timeout_seconds: i64, allow_remote_merge: bool) -> Self {
        Self {
            timeout_duration: Duration::seconds(timeout_seconds),
            allow_remote_merge,
            requests: Mutex::new(HashMap::new()),
            counter: Mutex::new(1),
        }
    }

    pub fn create_request(
        &self,
        task_id: &str,
        action: &str,
        is_merge_action: bool,
    ) -> PendingApproval {
        let mut count = self.counter.lock().unwrap();
        let request_id = format!("req_{}_{}", Utc::now().timestamp_millis(), *count);
        *count += 1;

        let now = Utc::now();
        let expires_at = now + self.timeout_duration;

        let approval = PendingApproval {
            request_id: request_id.clone(),
            task_id: task_id.to_string(),
            action: action.to_string(),
            is_merge_action,
            created_at: now,
            expires_at,
            decision: None,
        };

        self.requests
            .lock()
            .unwrap()
            .insert(request_id, approval.clone());
        approval
    }

    pub fn resolve(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> Result<(), ApprovalError> {
        let mut map = self.requests.lock().unwrap();
        let req = map
            .get_mut(request_id)
            .ok_or_else(|| ApprovalError::NotFound(request_id.to_string()))?;

        if req.decision.is_some() {
            return Err(ApprovalError::AlreadyResolved(request_id.to_string()));
        }

        let now = Utc::now();
        if now > req.expires_at {
            req.decision = Some(ApprovalDecision::Deny);
            return Err(ApprovalError::Expired(request_id.to_string()));
        }

        // Check remote merge policy
        if decision == ApprovalDecision::Approve && req.is_merge_action && !self.allow_remote_merge
        {
            req.decision = Some(ApprovalDecision::Deny);
            return Err(ApprovalError::RemoteMergeDisabled);
        }

        req.decision = Some(decision);
        Ok(())
    }

    pub fn get_status(&self, request_id: &str) -> Option<PendingApproval> {
        self.requests.lock().unwrap().get(request_id).cloned()
    }
}
