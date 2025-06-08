use serde::{Deserialize, Serialize};
use log::{info, error};
use serde_json::Value;

const OLLAMA_CHAT_API_URL: &str = "http://localhost:11434/api/chat";


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Serialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Serialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub tools: Vec<Tool>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    #[allow(dead_code)]
    pub model: String,
    pub response: String,
    #[allow(dead_code)]
    pub done: bool,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    #[allow(dead_code)]
    pub model: String,
    pub message: ChatMessage,
    #[allow(dead_code)]
    pub done: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("Failed to send request to Ollama: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Ollama API error: {0}")]
    ApiError(String),
}

pub struct OllamaClient {
    client: reqwest::Client
}

impl OllamaClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new()
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>, model: String, tools: Vec<Tool>) -> Result<ChatResponse, OllamaError> {
        info!("Sending chat request to Ollama with model: {}", model);
        
        let request = ChatRequest {
            model,
            messages,
            stream: false,
            tools,
        };

        let response = self
            .client
            .post(OLLAMA_CHAT_API_URL)
            .json(&request)
            .send()
            .await
            .map_err(OllamaError::RequestError)?;

        if !response.status().is_success() {
            let error_msg = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Ollama API error: {}", error_msg);
            return Err(OllamaError::ApiError(error_msg));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(OllamaError::RequestError)?;

        info!("Received response from Ollama chat");
        Ok(chat_response)
    }
}
