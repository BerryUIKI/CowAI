use crate::provider::LlmProvider;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};
use async_trait::async_trait;
use cow_core::error::CowResult;
use std::sync::Mutex;

/// Mock LLM provider returning predefined responses or echo responses for tests.
pub struct MockLlmProvider {
    responses: Mutex<Vec<ChatCompletionResponse>>,
}

impl MockLlmProvider {
    pub fn new(responses: Vec<ChatCompletionResponse>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }

    pub fn echo() -> Self {
        Self::new(Vec::new())
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn chat_complete(
        &self,
        request: &ChatCompletionRequest,
    ) -> CowResult<ChatCompletionResponse> {
        let mut queue = self.responses.lock().unwrap();
        if !queue.is_empty() {
            return Ok(queue.remove(0));
        }

        // Default echo behavior
        let last_msg = request
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or("");
        Ok(ChatCompletionResponse {
            content: Some(format!("Mock response to: {}", last_msg)),
            tool_calls: Vec::new(),
            finish_reason: Some("stop".to_string()),
        })
    }
}
