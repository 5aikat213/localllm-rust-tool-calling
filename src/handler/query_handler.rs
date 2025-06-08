use actix_web::{web, HttpResponse, Error};
use chrono::Local;
use serde::{Deserialize, Serialize};
use log::{info, error};
use std::fs;

use crate::llm::ollama::{OllamaClient, ChatMessage, Tool, ChatResponse};
use crate::tools::{WebSearchClient, PythonInvoker};

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
    python_invoker: PythonInvoker,
    system_prompt: String,
}

impl QueryHandler {
    pub fn new() -> Self {
        let system_prompt = fs::read_to_string("src/handler/system_prompt.txt").unwrap_or_else(|e| {
            error!("Failed to read system_prompt.txt: {}. Using default prompt.", e);
            "You are a helpful assistant.".to_string()
        });
        Self {
            ollama_client: OllamaClient::new(),
            search_client: WebSearchClient::new(),
            python_invoker: PythonInvoker::new(),
            system_prompt,
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
            function: crate::llm::ollama::ToolFunction {
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

    fn create_python_invoker_tool() -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: crate::llm::ollama::ToolFunction {
                name: "python_invoker".to_string(),
                description: "Executes a python script provided as a string and returns its output.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "script": {
                            "type": "string",
                            "description": "The Python script to execute."
                        },
                        "args": {
                            "type": "array",
                            "description": "Optional arguments to pass to the script.",
                            "items": {
                                "type": "string"
                            }
                        }
                    },
                    "required": ["script"]
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
                let tool_name = tool_call.function.name.as_str();
                let args = &tool_call.function.arguments;

                match tool_name {
                    "websearch" => {
                        if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
                            let count = args.get("count")
                                .and_then(|c| c.as_u64())
                                .unwrap_or(5) as usize;

                            match self.search_client.search(query.to_string(), count).await {
                                Ok(results) => {
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
                    "python_invoker" => {
                        if let Some(script) = args.get("script").and_then(|s| s.as_str()) {
                            let script_args: Vec<&str> = args.get("args")
                                .and_then(|a| a.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                                .unwrap_or_else(Vec::new);
                            
                            match self.python_invoker.run_script(script, &script_args) {
                                Ok(result) => {
                                    let response = format!("Exit Code: {:?}\nStdout: {}\nStderr: {}", result.exit_code, result.stdout, result.stderr);
                                    return Ok(Some((tool_call.function.name.clone(), response)));
                                }
                                Err(e) => {
                                    error!("Python invoker error: {}", e);
                                    return Err(format!("Python script execution failed: {}", e));
                                }
                            }
                        }
                    }
                    _ => {
                        // Unknown tool
                    }
                }
            }
        }

        Ok(None)
    }

    /// Handles chat requests by processing the message and interacting with the Ollama client.
    pub async fn handle_chat(&self, req: web::Json<ChatRequest>) -> Result<HttpResponse, Error> {
        info!("Processing chat request for model: {}", req.model);
        
        let now = Local::now();
        let formatted_datetime = now.to_rfc3339();
        let system_prompt = format!("{} Current date and time: {}", self.system_prompt, formatted_datetime);

        let mut messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
                tool_calls: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: req.message.clone(),
                tool_calls: None,
            }
        ];

        let mut response = String::new();
        
        loop {
            // Call Ollama with the messages and websearch tool
            let chat_response = match self.ollama_client
                .chat(messages.clone(), req.model.clone(), vec![Self::create_websearch_tool(), Self::create_python_invoker_tool()])
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
