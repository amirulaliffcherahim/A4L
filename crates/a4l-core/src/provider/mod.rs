pub mod config;
pub mod mock;
pub mod openai;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use config::{ProviderConfig, ProvidersFile};
pub use mock::MockProvider;
pub use openai::OpenAiProvider;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("network or http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json serialize/deserialize error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("provider returned error: {0}")]
    Api(String),
    #[error("stream error: {0}")]
    Stream(String),
    #[error("no mock response queued")]
    MockExhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant" | "tool"
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String, // "function"
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String, // "function"
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub cached_prompt_tokens: Option<u32>,
    #[serde(default)]
    pub is_estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub message: ChatMessage,
    pub usage: Usage,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse, ProviderError>;
}
