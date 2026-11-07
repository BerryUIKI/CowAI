use crate::types::{ChatCompletionRequest, ChatCompletionResponse};
use async_trait::async_trait;
use cowbot_core::error::{CowError, CowResult};

/// Trait implemented by LLM backend providers.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider identification name (e.g. "openai", "deepseek", "ollama", "mock")
    fn name(&self) -> &str;

    /// Generates a chat completion.
    async fn chat_complete(
        &self,
        request: &ChatCompletionRequest,
    ) -> CowResult<ChatCompletionResponse>;

    /// Lists the model ids the endpoint reports.
    ///
    /// Providers without a usable catalogue keep the default, which reports
    /// model discovery as unsupported so callers can fall back to manual entry.
    async fn list_models(&self) -> CowResult<Vec<String>> {
        Err(CowError::Llm(format!(
            "{} does not support model discovery",
            self.name()
        )))
    }
}
