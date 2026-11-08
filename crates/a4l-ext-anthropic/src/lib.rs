use a4l_core::provider::{
    ChatMessage, LlmProvider, ProviderError, ProviderResponse, ToolDefinition, Usage,
};
use a4l_ext_api::{Extension, Registry};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::json;

pub struct AnthropicExtension {
    pub api_key: Option<String>,
}

impl Default for AnthropicExtension {
    fn default() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        }
    }
}

impl Extension for AnthropicExtension {
    fn name(&self) -> &str {
        "anthropic-native-provider"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn register(&self, _registry: &mut Registry) {
        // Registers native Anthropic capability
    }
}

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    pub fn build_messages_payload(
        &self,
        messages: &[ChatMessage],
        _tools: &[ToolDefinition],
    ) -> serde_json::Value {
        let mut system_text = String::new();
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                if let Some(content) = &msg.content {
                    system_text.push_str(content);
                    system_text.push_str("\n\n");
                }
            } else {
                let role = if msg.role == "assistant" {
                    "assistant"
                } else {
                    "user"
                };
                anthropic_messages.push(json!({
                    "role": role,
                    "content": msg.content.as_deref().unwrap_or("")
                }));
            }
        }

        let mut payload = json!({
            "model": self.model,
            "max_tokens": 4096,
            "messages": anthropic_messages,
        });

        if !system_text.is_empty() {
            // Apply cache_control to system prompt for ephemeral prefix caching
            payload["system"] = json!([
                {
                    "type": "text",
                    "text": system_text.trim(),
                    "cache_control": { "type": "ephemeral" }
                }
            ]);
        }

        payload
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse, ProviderError> {
        let payload = self.build_messages_payload(messages, tools);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(
            "anthropic-beta",
            HeaderValue::from_static("prompt-caching-2024-07-31"),
        );

        if let Ok(val) = HeaderValue::from_str(&self.api_key) {
            headers.insert("x-api-key", val);
        }

        let res = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("HTTP {}: {}", status, text)));
        }

        let raw: serde_json::Value = res.json().await?;

        // Extract assistant text
        let content_text = raw
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let prompt_tokens = raw
            .get("usage")
            .and_then(|u| u.get("input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let completion_tokens = raw
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let cached_tokens = raw
            .get("usage")
            .and_then(|u| u.get("cache_read_input_tokens"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        Ok(ProviderResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: Some(content_text.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            usage: Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                cached_prompt_tokens: cached_tokens,
                is_estimated: false,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_extension_registration() {
        let ext = AnthropicExtension::default();
        assert_eq!(ext.name(), "anthropic-native-provider");
        let mut reg = Registry::new();
        ext.register(&mut reg);
    }

    #[test]
    fn test_anthropic_messages_payload_and_cache_control() {
        let provider = AnthropicProvider::new("sk-ant-test", "claude-3-5-sonnet-20241022");
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some("Static Kernel Instruction".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some("Implement metrics".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let payload = provider.build_messages_payload(&messages, &[]);
        assert_eq!(payload["model"], "claude-3-5-sonnet-20241022");

        // Verify system prompt carries ephemeral cache_control
        let sys_arr = payload["system"].as_array().unwrap();
        assert_eq!(sys_arr.len(), 1);
        assert_eq!(sys_arr[0]["cache_control"]["type"], "ephemeral");
        assert_eq!(sys_arr[0]["text"], "Static Kernel Instruction");

        // Verify user turn
        let user_msgs = payload["messages"].as_array().unwrap();
        assert_eq!(user_msgs.len(), 1);
        assert_eq!(user_msgs[0]["role"], "user");
        assert_eq!(user_msgs[0]["content"], "Implement metrics");
    }
}
