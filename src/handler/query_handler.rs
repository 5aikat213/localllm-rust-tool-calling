use actix_web::{web, HttpResponse, Error};
use serde::{Deserialize, Serialize};
use log::{info, error};

use crate::model::ollama::{OllamaClient, ChatMessage, Tool, ChatResponse};
use crate::tools::WebSearchClient;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct ChatApiResponse {
    pub response: String,
}

pub struct QueryHandler {
    ollama_client: OllamaClient,
    search_client: WebSearchClient,
}

impl QueryHandler {
    pub fn new() -> Self {
        Self {
            ollama_client: OllamaClient::new(),
            search_client: WebSearchClient::new(),
        }
    }

    /**
        * Creates a websearch tool for the Ollama client.
        * This tool allows the model to perform web searches for the latest events and news.
        * The tool is defined with a name, description, and parameters.
        * The parameters include a query string and an optional count for the number of search results.
        * The function returns a Tool object that can be used in the Ollama client.
     */
    fn create_websearch_tool() -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: crate::model::ollama::ToolFunction {
                name: "websearch".to_string(),
                description: "Get search results from web for latest events, news.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query to do web search on."
                        },
                        "count": {
                            "type": "number",
                            "description": "Optional field to mention how many web search results are needed"
                        }
                    },
                    "required": ["query"]
                }),
            },
        }
    }

    /**
        * Processes tool calls in the chat response.
        * If the tool call is for the websearch tool, it performs a web search using the WebSearchClient.
        * The search results are formatted and returned as a string.
        * The function returns a Result containing the tool name and the search results as a string.
        * If the tool call is not for the websearch tool, it returns None.
        * If there is an error during the web search, it returns an error string.
     */
    async fn process_tool_calls(&self, chat_response: &ChatResponse) -> Result<Option<(String, String)>, String> {
        if let Some(tool_calls) = &chat_response.message.tool_calls {
            for tool_call in tool_calls {
                if tool_call.function.name == "websearch" {
                    let args = &tool_call.function.arguments;
                    
                    // Extract query and optional count
                    if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
                        let count = args.get("count")
                            .and_then(|c| c.as_u64())
                            .unwrap_or(5) as usize;

                        // Perform web search
                        match self.search_client.search(query.to_string(), count).await {
                            Ok(results) => {
                                // Format search results
                                let results_text = results.iter()
                                    .map(|r| format!("Title: {}\nURL: {}\nContent: {}\n---", 
                                        r.title, r.url, r.content))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                
                                return Ok(Some((tool_call.function.name.clone(), results_text)));
                            }
                            Err(e) => {
                                error!("Web search error: {}", e);
                                return Err(format!("Web search failed: {}", e));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Handles chat requests by processing the message and interacting with the Ollama client.
    pub async fn handle_chat(&self, req: web::Json<ChatRequest>) -> Result<HttpResponse, Error> {
        info!("Processing chat request for model: {}", req.model);

        let mut messages = vec![ChatMessage {
            role: "user".to_string(),
            content: req.message.clone(),
            tool_calls: None,
        }];

        let mut response = String::new();
        
        loop {
            // Call Ollama with the messages and websearch tool
            let chat_response = match self.ollama_client
                .chat(messages.clone(), req.model.clone(), vec![Self::create_websearch_tool()])
                .await {
                    Ok(response) => response,
                    Err(e) => {
                        error!("Ollama chat error: {}", e);
                        return Ok(HttpResponse::InternalServerError().json(ChatApiResponse {
                            response: format!("Error: {}", e),
                        }));
                    }
                };
            
            info!("Tool calls: {:?}", chat_response.message.tool_calls);
            // Process any tool calls in the response
            match self.process_tool_calls(&chat_response).await {
                Ok(Some((_, tool_output))) => {
                    // Add assistant message
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: chat_response.message.content.clone(),
                        tool_calls: chat_response.message.tool_calls.clone(),
                    });

                    // Add tool message
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: tool_output,
                        tool_calls: None,
                    });

                    // Continue the loop to process the tool response
                    continue;
                }
                Ok(None) => {
                    // No more tool calls, use the final message content
                    info!("Final response recieved from the model.");
                    response = chat_response.message.content;
                    break;
                }
                Err(e) => {
                    error!("Tool processing error: {}", e);
                    return Ok(HttpResponse::InternalServerError().json(ChatApiResponse {
                        response: format!("Error: {}", e),
                    }));
                }
            }
        }

        Ok(HttpResponse::Ok().json(ChatApiResponse {
            response,
        }))
    }
}
