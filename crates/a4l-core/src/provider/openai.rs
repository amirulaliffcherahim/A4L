use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;

use super::{
    ChatMessage, LlmProvider, ProviderConfig, ProviderError, ProviderResponse, ToolDefinition,
    Usage,
};

pub struct OpenAiProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    api_key: Option<String>,
}

impl OpenAiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let api_key = if let Some(env_var) = &config.api_key_env {
            std::env::var(env_var).ok()
        } else {
            None
        };

        Self {
            config,
            client: reqwest::Client::new(),
            api_key,
        }
    }

    fn estimate_tokens(text: &str) -> u32 {
        // Simple 4-chars per token heuristic
        ((text.len() as f64) / 4.0).ceil() as u32
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse, ProviderError> {
        let mut url = self.config.base_url.trim_end_matches('/').to_string();
        if !url.ends_with("/chat/completions") {
            url.push_str("/chat/completions");
        }

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(key) = &self.api_key {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", key)) {
                headers.insert(AUTHORIZATION, val);
            }
        }

        for (k, v) in &self.config.extra_headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(k.as_bytes()),
                HeaderValue::from_str(v),
            ) {
                headers.insert(name, val);
            }
        }

        let mut body = json!({
            "model": self.config.default_model,
            "messages": messages,
            "stream": false
        });

        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("HTTP {}: {}", status, text)));
        }

        let raw_json: serde_json::Value = response.json().await?;

        // Parse choices[0].message
        let choice_msg = raw_json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c0| c0.get("message"))
            .ok_or_else(|| {
                ProviderError::Api("missing choices[0].message in response".to_string())
            })?;

        let parsed_msg: ChatMessage = serde_json::from_value(choice_msg.clone())?;

        // Parse or estimate usage
        let usage = if let Some(usage_val) = raw_json.get("usage") {
            Usage {
                prompt_tokens: usage_val
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: usage_val
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: usage_val
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                cached_prompt_tokens: usage_val
                    .get("prompt_tokens_details")
                    .and_then(|d| d.get("cached_tokens"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
                is_estimated: false,
            }
        } else {
            // Local token estimation heuristic
            let prompt_chars: usize = messages
                .iter()
                .map(|m| m.content.as_deref().unwrap_or("").len())
                .sum();
            let completion_chars = parsed_msg.content.as_deref().unwrap_or("").len();
            let pt = Self::estimate_tokens(&prompt_chars.to_string());
            let ct = Self::estimate_tokens(&completion_chars.to_string());
            Usage {
                prompt_tokens: pt,
                completion_tokens: ct,
                total_tokens: pt + ct,
                cached_prompt_tokens: None,
                is_estimated: true,
            }
        };

        Ok(ProviderResponse {
            message: parsed_msg,
            usage,
        })
    }
}
