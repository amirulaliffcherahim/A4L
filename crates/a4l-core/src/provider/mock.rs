use async_trait::async_trait;
use std::sync::Mutex;

use super::{ChatMessage, LlmProvider, ProviderError, ProviderResponse, ToolDefinition, Usage};

pub struct MockProvider {
    responses: Mutex<Vec<ProviderResponse>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
        }
    }

    pub fn with_responses(responses: Vec<ProviderResponse>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }

    pub fn queue_text_response(&self, text: &str) {
        let resp = ProviderResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: Some(text.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
                cached_prompt_tokens: None,
                is_estimated: false,
            },
        };
        self.responses.lock().unwrap().push(resp);
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(
        &self,
        _messages: &[ChatMessage],
        _tools: &[ToolDefinition],
    ) -> Result<ProviderResponse, ProviderError> {
        let mut queue = self.responses.lock().unwrap();
        if queue.is_empty() {
            return Err(ProviderError::MockExhausted);
        }
        Ok(queue.remove(0))
    }
}
