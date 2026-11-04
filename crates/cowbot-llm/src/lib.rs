pub mod mock;
pub mod openai;
pub mod provider;
pub mod types;

pub use mock::MockLlmProvider;
pub use openai::OpenAiCompatibleProvider;
pub use provider::LlmProvider;
pub use types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatRole, LlmToolCall,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockLlmProvider::echo();
        let req = ChatCompletionRequest {
            model: "mock-model".to_string(),
            messages: vec![ChatMessage::user("Hello CowBot")],
            tools: Vec::new(),
            temperature: None,
        };
        let res = provider.chat_complete(&req).await.unwrap();
        assert_eq!(res.content.unwrap(), "Mock response to: Hello CowBot");
    }
}
