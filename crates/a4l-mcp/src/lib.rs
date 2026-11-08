pub mod client;
pub mod config;

use a4l_policy::{ActionRequest, PolicyDecision, PolicyEngine};
use serde_json::Value;
use thiserror::Error;

pub use client::{McpError, McpToolInfo, StdioMcpClient};
pub use config::{McpConfigFile, McpServerConfig};

#[derive(Debug, Error)]
pub enum McpToolError {
    #[error("mcp error: {0}")]
    Mcp(#[from] McpError),
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("policy confirmation required: {0}")]
    PolicyAskRequired(String),
}

pub struct McpToolBridge<'a> {
    pub server_name: String,
    pub tool_info: McpToolInfo,
    pub policy_engine: &'a PolicyEngine,
}

impl<'a> McpToolBridge<'a> {
    pub fn new(
        server_name: impl Into<String>,
        tool_info: McpToolInfo,
        policy_engine: &'a PolicyEngine,
    ) -> Self {
        Self {
            server_name: server_name.into(),
            tool_info,
            policy_engine,
        }
    }

    pub fn check_call_policy(&self, args: &Value) -> Result<(), McpToolError> {
        // Form action request
        let req = ActionRequest::Command {
            cmd: format!("mcp:{}:{} {}", self.server_name, self.tool_info.name, args),
        };

        match self.policy_engine.evaluate(&req) {
            PolicyDecision::Allow => Ok(()),
            PolicyDecision::Ask(reason) => Err(McpToolError::PolicyAskRequired(reason)),
            PolicyDecision::Deny(reason) => Err(McpToolError::PolicyDenied(reason)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4l_policy::{PolicyEngine, Role};
    use serde_json::json;

    #[test]
    fn test_mcp_config_parsing() {
        let json_str = r#"{
            "mcpServers": {
                "sqlite": {
                    "command": "uvx",
                    "args": ["mcp-server-sqlite", "--db-path", "test.db"],
                    "env": {}
                }
            }
        }"#;

        let parsed: McpConfigFile = serde_json::from_str(json_str).unwrap();
        assert!(parsed.mcp_servers.contains_key("sqlite"));
        let server = &parsed.mcp_servers["sqlite"];
        assert_eq!(server.command, "uvx");
        assert_eq!(server.args.len(), 3);
    }

    #[test]
    fn test_mcp_tool_policy_routing() {
        let engine = PolicyEngine::new(Role::Maker);
        let tool = McpToolInfo {
            name: "execute_query".to_string(),
            description: "Run SQL query".to_string(),
            input_schema: json!({}),
        };

        let bridge = McpToolBridge::new("sqlite", tool, &engine);
        // Default policy for unknown command is ask
        let res = bridge.check_call_policy(&json!({"query": "SELECT * FROM users"}));
        assert!(matches!(res, Err(McpToolError::PolicyAskRequired(_))));
    }
}
