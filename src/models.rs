use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saying {
    pub id: String,
    pub content: String,
    pub prompt: String,
    pub created_at: DateTime<Utc>,
    pub source: SayingSource,
    pub preset_id: Option<String>, // Track which preset was used, if any
}

// Global cache key for identifying reusable sayings across users
#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
pub struct CacheKey {
    pub preset_id: Option<String>,
    pub prompt: String,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.preset_id == other.preset_id && self.prompt == other.prompt
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.preset_id.hash(state);
        self.prompt.hash(state);
    }
}

impl CacheKey {
    pub fn new(preset_id: Option<String>, prompt: String) -> Self {
        Self { preset_id, prompt }
    }
    
    // Create from a saying
    pub fn from_saying(saying: &Saying) -> Self {
        Self {
            preset_id: saying.preset_id.clone(),
            prompt: saying.prompt.clone(),
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenRouterUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterChoice {
    pub message: OpenRouterMessage,
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterMessage {
    pub content: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}