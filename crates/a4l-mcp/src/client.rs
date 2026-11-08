use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Error)]
pub enum McpError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

pub struct StdioMcpClient {
    child: Child,
    next_id: u64,
}

impl StdioMcpClient {
    pub fn spawn(command: &str, args: &[String]) -> Result<Self, McpError> {
        let child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(Self { child, next_id: 1 })
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpToolInfo>, McpError> {
        let id = self.next_id;
        self.next_id += 1;

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: "tools/list".to_string(),
            params: json!({}),
        };

        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| McpError::Protocol("child stdin missing".to_string()))?;
        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| McpError::Protocol("child stdout missing".to_string()))?;

        let line = serde_json::to_string(&req)? + "\n";
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;

        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let resp: JsonRpcResponse = serde_json::from_str(&response_line)?;
        if let Some(err) = resp.error {
            return Err(McpError::Protocol(format!("RPC error: {:?}", err)));
        }

        let tools = resp
            .result
            .and_then(|r| r.get("tools").cloned())
            .ok_or_else(|| McpError::Protocol("missing tools in response".to_string()))?;

        let list: Vec<McpToolInfo> = serde_json::from_value(tools)?;
        Ok(list)
    }
}
