use crate::types::{ChatCompletionRequest, ChatCompletionResponse};
use async_trait::async_trait;
use berry_core::error::BerryResult;

/// Trait implemented by LLM backend providers.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider identification name (e.g. "openai", "deepseek", "ollama", "mock")
    fn name(&self) -> &str;

    /// Generates a chat completion.
    async fn chat_complete(
        &self,
        request: &ChatCompletionRequest,
    ) -> BerryResult<ChatCompletionResponse>;
}
