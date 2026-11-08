use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    TaskStateChanged {
        task_id: String,
        role: String,
        status: String,
        timestamp: DateTime<Utc>,
    },
    GateResultReported {
        task_id: String,
        tier: String,
        exit: Option<i32>,
        timestamp: DateTime<Utc>,
    },
    ApprovalRequested {
        request_id: String,
        task_id: String,
        action: String,
        expires_at: DateTime<Utc>,
    },
    QuestionRequested {
        request_id: String,
        task_id: Option<String>,
        question: String,
        options: Vec<String>,
    },
    ErrorOccurred {
        task_id: Option<String>,
        error: String,
        timestamp: DateTime<Utc>,
    },
}
