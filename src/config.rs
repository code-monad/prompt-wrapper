use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub openrouter: OpenRouterConfig,
    pub rate_limit: RateLimitConfig,
    pub response_cache: ResponseCacheConfig,
    pub storage: StorageConfig,
    pub presets: PresetsConfig,
    pub bitcoin: BitcoinConfig,
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
pub struct ResponseCacheConfig {
    pub refresh_seconds: i64,
    pub excluded_user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetsConfig {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinConfig {
    pub btcpay_api_url: String,
    pub btcpay_api_key: String,
    pub rpc_url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    #[serde(rename = "sqlite")]
    SQLite,
    #[serde(rename = "redis")]
    Redis,
    #[serde(rename = "memory")]
    Memory,
    #[serde(rename = "sled")]
    Sled,
}

// Test user ID - only valid in debug builds
#[cfg(debug_assertions)]
pub const TEST_USER_ID: &str = "test_user";

#[cfg(not(debug_assertions))]
pub const TEST_USER_ID: &str = "invalid_test_user";

impl Config {
    pub fn from_env() -> Self {
        let refresh_seconds = env::var("USER_RESPONSE_REFRESH_SECONDS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0)
            .max(0);

        let excluded_user_ids = env::var("USER_RESPONSE_REFRESH_EXCLUDE_LIST")
            .unwrap_or_default()
            .split(',')
            .filter_map(|raw| {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect::<Vec<_>>();

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
                model: env::var("OPENROUTER_MODEL")
                    .unwrap_or_else(|_| "mistralai/mistral-7b-instruct".to_string()),
                base_url: env::var("OPENROUTER_BASE_URL")
                    .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string()),
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
            response_cache: ResponseCacheConfig {
                refresh_seconds,
                excluded_user_ids,
            },
            storage: StorageConfig {
                type_: match env::var("STORAGE_TYPE")
                    .unwrap_or_else(|_| "memory".to_string())
                    .as_str()
                {
                    "sqlite" => StorageType::SQLite,
                    "redis" => StorageType::Redis,
                    "sled" => StorageType::Sled,
                    _ => StorageType::Memory,
                },
                connection_string: env::var("STORAGE_CONNECTION_STRING")
                    .unwrap_or_else(|_| "memory".to_string()),
            },
            presets: PresetsConfig {
                file_path: env::var("PRESETS_FILE_PATH")
                    .unwrap_or_else(|_| "./presets.yaml".to_string()),
            },
            bitcoin: BitcoinConfig {
                btcpay_api_url: env::var("BTCPAY_API_URL")
                    .unwrap_or_else(|_| "https://btcpay.nvap.link/api/v1/server/info".to_string()),
                btcpay_api_key: env::var("BTCPAY_API_KEY")
                    .unwrap_or_else(|_| "ebf4a2293b1a1ed3ab1b28706f0a937cc21fbbac".to_string()),
                rpc_url: env::var("BITCOIN_RPC_URL")
                    .unwrap_or_else(|_| "https://bitcoin-rpc.publicnode.com".to_string()),
            },
        }
    }
}
