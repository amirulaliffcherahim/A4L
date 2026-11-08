use a4l_policy::{ActionRequest, PolicyDecision, PolicyEngine};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("execution error: {0}")]
    Execution(String),
    #[error("incompatible api version: extension requires {required}, host is {host}")]
    IncompatibleVersion { required: String, host: String },
}

pub const API_VERSION: &str = "0.1.0";

/// Host Handle passed to extensions for operations that must respect policy
#[derive(Clone)]
pub struct HostHandle {
    policy_engine: Arc<PolicyEngine>,
}

impl HostHandle {
    pub fn new(policy_engine: Arc<PolicyEngine>) -> Self {
        Self { policy_engine }
    }

    pub fn check_policy(&self, request: &ActionRequest) -> Result<(), ExtensionError> {
        match self.policy_engine.evaluate(request) {
            PolicyDecision::Allow => Ok(()),
            PolicyDecision::Ask(reason) => Err(ExtensionError::PolicyDenied(format!(
                "interactive confirmation required: {}",
                reason
            ))),
            PolicyDecision::Deny(reason) => Err(ExtensionError::PolicyDenied(reason)),
        }
    }
}

/// Custom Tool interface provided by extensions
#[async_trait]
pub trait CustomTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn call(
        &self,
        host: &HostHandle,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, ExtensionError>;
}

/// Lifecycle Hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent<'a> {
    BeforeToolCall {
        tool: &'a str,
        args: &'a str,
    },
    OnStop {
        task_id: &'a str,
        all_gates_green: bool,
    },
    OnRoleTransition {
        task_id: &'a str,
        from_role: &'a str,
        to_role: &'a str,
    },
    OnCircuitBreakerTrip {
        task_id: &'a str,
        reason: &'a str,
    },
    OnHandoffBuild {
        task_id: &'a str,
    },
}

#[async_trait]
pub trait LifecycleHook: Send + Sync {
    async fn on_event(&self, event: &LifecycleEvent<'_>) -> Result<(), ExtensionError>;
}

/// Chat Channel interface (e.g. Telegram, Discord, Slack)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: String,
    pub task_id: String,
    pub action_description: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub sender_id: String,
    pub text: String,
}

#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn is_sender_allowed(&self, sender_id: &str) -> bool;
    async fn send_status_card(&self, title: &str, body: &str) -> Result<(), ExtensionError>;
    async fn send_approval_request(&self, request: &ApprovalRequest) -> Result<(), ExtensionError>;
}

/// Extension Registry
#[derive(Default)]
pub struct Registry {
    pub tools: HashMap<String, Box<dyn CustomTool>>,
    pub hooks: Vec<Box<dyn LifecycleHook>>,
    pub channels: HashMap<String, Box<dyn Channel>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_tool(&mut self, tool: Box<dyn CustomTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn register_hook(&mut self, hook: Box<dyn LifecycleHook>) {
        self.hooks.push(hook);
    }

    pub fn register_channel(&mut self, channel: Box<dyn Channel>) {
        self.channels.insert(channel.name().to_string(), channel);
    }
}

/// The Extension Trait that third-party extensions must implement
pub trait Extension: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn register(&self, registry: &mut Registry);
}
