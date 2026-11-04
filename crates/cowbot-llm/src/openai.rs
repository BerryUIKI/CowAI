use crate::provider::LlmProvider;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse, LlmToolCall};
use async_trait::async_trait;
use cowbot_core::error::{CowError, CowResult};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error};

/// Generic client for any OpenAI-compatible API endpoint.
#[derive(Clone)]
pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
        }
    }

    pub fn openai(api_key: String) -> Self {
        Self::new("https://api.openai.com/v1", Some(api_key))
    }

    pub fn deepseek(api_key: String) -> Self {
        Self::new("https://api.deepseek.com", Some(api_key))
    }

    pub fn ollama(base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "http://localhost:11434/v1".to_string());
        Self::new(url, None)
    }
}

#[derive(Serialize)]
struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: &'a [crate::types::ChatMessage],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiRawToolCall>>,
}

#[derive(Deserialize)]
struct OpenAiRawToolCall {
    id: String,
    function: OpenAiRawFunction,
}

#[derive(Deserialize)]
struct OpenAiRawFunction {
    name: String,
    arguments: String,
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        "openai-compatible"
    }

    async fn chat_complete(
        &self,
        request: &ChatCompletionRequest,
    ) -> CowResult<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(key) = &self.api_key {
            let auth_val = format!("Bearer {}", key);
            if let Ok(hv) = HeaderValue::from_str(&auth_val) {
                headers.insert(AUTHORIZATION, hv);
            }
        }

        // Format tools in OpenAI JSON schema format
        let tools: Vec<serde_json::Value> = request
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect();

        let req_body = OpenAiChatRequest {
            model: &request.model,
            messages: &request.messages,
            tools,
            temperature: request.temperature,
        };

        debug!(url = %url, model = %request.model, "Dispatching chat completion request");

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| CowError::Llm(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            error!(status = %status, body = %err_text, "LLM provider returned non-200 status");
            return Err(CowError::Llm(format!(
                "API returned error status {}: {}",
                status, err_text
            )));
        }

        let resp_json: OpenAiChatResponse = response
            .json()
            .await
            .map_err(|e| CowError::Llm(format!("Failed to parse response JSON: {}", e)))?;

        let choice = resp_json
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| CowError::Llm("No choices in chat response".to_string()))?;

        let mut tool_calls = Vec::new();
        if let Some(raw_calls) = choice.message.tool_calls {
            for raw in raw_calls {
                let args_json = serde_json::from_str(&raw.function.arguments)
                    .unwrap_or_else(|_| json!({ "raw": raw.function.arguments }));
                tool_calls.push(LlmToolCall {
                    id: raw.id,
                    name: raw.function.name,
                    arguments: args_json,
                });
            }
        }

        Ok(ChatCompletionResponse {
            content: choice.message.content,
            tool_calls,
            finish_reason: choice.finish_reason,
        })
    }
}
