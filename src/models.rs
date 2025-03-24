use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saying {
    pub id: String,
    pub content: String,
    pub prompt: String,
    pub created_at: DateTime<Utc>,
    pub source: SayingSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SayingSource {
    #[serde(rename = "llm")]
    LLM,
    #[serde(rename = "cache")]
    Cache,
    #[serde(rename = "database")]
    Database,
}

impl std::fmt::Display for SayingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SayingSource::LLM => write!(f, "llm"),
            SayingSource::Cache => write!(f, "cache"),
            SayingSource::Database => write!(f, "database"),
        }
    }
}

impl From<SayingSource> for String {
    fn from(source: SayingSource) -> Self {
        source.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub user_id: String,
    pub remaining_requests: u32,
    pub reset_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterResponse {
    pub id: String,
    pub choices: Vec<OpenRouterChoice>,
    pub created: i64,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterChoice {
    pub message: OpenRouterMessage,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterMessage {
    pub content: String,
    pub role: String,
} 