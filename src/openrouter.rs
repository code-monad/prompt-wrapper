use anyhow::{Result, Context, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::OpenRouterConfig;
use crate::models::{OpenRouterResponse, Saying, SayingSource};

#[derive(Debug, Clone)]
pub struct OpenRouterClient {
    config: OpenRouterConfig,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: Option<String>,
    pub error: Option<String>,
}

impl OpenRouterClient {
    pub fn new(config: OpenRouterConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn get_saying(&self, prompt: &str) -> Result<Saying> {
        // Use default system prompt
        self.get_saying_with_system(
            "You are a helpful assistant that provides wise and thoughtful sayings.",
            prompt,
        ).await
    }

    pub async fn get_saying_with_system(&self, system_prompt: &str, user_prompt: &str) -> Result<Saying> {
        // Validate API key first
        if self.config.api_key.is_empty() {
            return Err(anyhow!("OpenRouter API key is not configured. Please add it to your .env file."));
        }

        let url = format!("{}/chat/completions", self.config.base_url);
        
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            },
        ];

        // Log the request for debugging
        tracing::debug!(
            "Sending request to OpenRouter with model: {} and messages: {:?}",
            self.config.model,
            serde_json::to_string(&messages).unwrap_or_default()
        );

        // Default model to use if none is specified (as in the TypeScript implementation)
        let model = if self.config.model.is_empty() {
            "openai/gpt-3.5-turbo".to_string()
        } else {
            self.config.model.clone()
        };

        let response_result = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            // Add headers similar to TypeScript implementation
            .header("HTTP-Referer", "http://localhost:3000")
            .header("X-Title", "AI Chat Tool")
            .json(&json!({
                "model": model,
                "messages": messages,
            }))
            .send()
            .await;

        // Handle request errors
        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Error sending request to OpenRouter: {}", e);
                return Err(anyhow!("Failed to connect to OpenRouter: {}", e));
            }
        };

        // Check status code first
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
            tracing::error!("OpenRouter API error: Status {}, Response: {}", status, error_text);
            return Err(anyhow!("OpenRouter API returned error {}: {}", status, error_text));
        }

        // Parse the response
        let response_data = match response.json::<OpenRouterResponse>().await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Error parsing OpenRouter response: {}", e);
                return Err(anyhow!("Failed to parse OpenRouter response: {}", e));
            }
        };

        // Extract the content from the first choice
        let content = if let Some(choice) = response_data.choices.first() {
            choice.message.content.clone()
        } else {
            return Err(anyhow!("OpenRouter response contained no choices"));
        };

        // Create a new Saying with default preset_id as None
        Ok(Saying {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            prompt: user_prompt.to_string(),
            created_at: chrono::Utc::now(),
            source: SayingSource::LLM,
            preset_id: None, // Will be set by the handler later
        })
    }

    // New method similar to TypeScript's generateChatResponse
    pub async fn generate_chat_response(&self, messages: Vec<Message>, model_id: Option<String>) -> ChatResponse {
        if self.config.api_key.is_empty() {
            return ChatResponse {
                content: None,
                error: Some("OpenRouter API key is not configured. Please add it to your .env file.".to_string()),
            };
        }

        let url = format!("{}/chat/completions", self.config.base_url);
        
        // Use provided model or default
        let model = model_id.unwrap_or_else(|| 
            if self.config.model.is_empty() { 
                "openai/gpt-3.5-turbo".to_string() 
            } else { 
                self.config.model.clone() 
            }
        );

        tracing::debug!(
            "Sending request to OpenRouter with model: {} and messages: {:?}",
            model,
            serde_json::to_string(&messages).unwrap_or_default()
        );

        // Execute the API call with error handling
        let response = match self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "http://localhost:3000")
            .header("X-Title", "AI Chat Tool")
            .json(&json!({
                "model": model,
                "messages": messages,
            }))
            .send()
            .await {
                Ok(res) => res,
                Err(e) => {
                    tracing::error!("Error calling OpenRouter API: {}", e);
                    return ChatResponse {
                        content: None,
                        error: Some(format!("Failed to connect to OpenRouter: {}", e)),
                    };
                }
            };

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("OpenRouter API error: {} - {}", status, error_text);
            return ChatResponse {
                content: None,
                error: Some(format!("OpenRouter API error ({}): {}", status, error_text)),
            };
        }

        // Parse JSON response
        let json_result = response.json::<OpenRouterResponse>().await;
        let json_response = match json_result {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to parse OpenRouter response: {}", e);
                return ChatResponse {
                    content: None,
                    error: Some(format!("Failed to parse OpenRouter response: {}", e)),
                };
            }
        };

        tracing::debug!("OpenRouter response: {:?}", serde_json::to_string(&json_response).unwrap_or_default());

        // Validate response structure similar to TypeScript implementation
        if json_response.choices.is_empty() || json_response.choices[0].message.content.is_empty() {
            tracing::error!("Invalid response from OpenRouter: {:?}", json_response);
            return ChatResponse {
                content: None,
                error: Some("Received an invalid response from OpenRouter.".to_string()),
            };
        }

        ChatResponse {
            content: Some(json_response.choices[0].message.content.clone()),
            error: None,
        }
    }
}