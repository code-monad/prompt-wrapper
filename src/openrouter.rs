use anyhow::Result;
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

impl OpenRouterClient {
    pub fn new(config: OpenRouterConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn get_saying(&self, prompt: &str) -> Result<Saying> {
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful assistant that provides wise and thoughtful sayings.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ];

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.config.model,
                "messages": messages,
            }))
            .send()
            .await?
            .json::<OpenRouterResponse>()
            .await?;

        let content = response.choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_else(|| "No response from LLM".to_string());

        Ok(Saying {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            prompt: prompt.to_string(),
            created_at: chrono::Utc::now(),
            source: SayingSource::LLM,
        })
    }
}