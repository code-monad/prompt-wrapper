use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub openrouter: OpenRouterConfig,
    pub rate_limit: RateLimitConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub type_: StorageType,
    pub connection_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    #[serde(rename = "sqlite")]
    SQLite,
    #[serde(rename = "redis")]
    Redis,
    #[serde(rename = "memory")]
    Memory,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse()
                    .unwrap_or(3000),
            },
            openrouter: OpenRouterConfig {
                api_key: env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set"),
                model: env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "mistralai/mistral-7b-instruct".to_string()),
                base_url: env::var("OPENROUTER_BASE_URL").unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string()),
            },
            rate_limit: RateLimitConfig {
                max_requests: env::var("RATE_LIMIT_MAX_REQUESTS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                window_seconds: env::var("RATE_LIMIT_WINDOW_SECONDS")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
            },
            storage: StorageConfig {
                type_: match env::var("STORAGE_TYPE").unwrap_or_else(|_| "memory".to_string()).as_str() {
                    "sqlite" => StorageType::SQLite,
                    "redis" => StorageType::Redis,
                    _ => StorageType::Memory,
                },
                connection_string: env::var("STORAGE_CONNECTION_STRING").unwrap_or_else(|_| "memory".to_string()),
            },
        }
    }
}