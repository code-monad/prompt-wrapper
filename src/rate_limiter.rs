use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::config::RateLimitConfig;
use crate::models::RateLimitInfo;

#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    // In a real application, you'd use a persistent store like Redis
    // This in-memory implementation is just for demonstration
    store: Arc<Mutex<HashMap<String, RateLimitInfo>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn check(&self, user_id: &str) -> Result<bool> {
        let mut store = self.store.lock().unwrap();
        let now = Utc::now();
        
        if let Some(info) = store.get(user_id) {
            // Check if the rate limit window has expired
            if now > info.reset_at {
                // Reset the rate limit
                let new_info = RateLimitInfo {
                    user_id: user_id.to_string(),
                    remaining_requests: self.config.max_requests - 1,
                    reset_at: now + Duration::seconds(self.config.window_seconds as i64),
                };
                store.insert(user_id.to_string(), new_info);
                return Ok(true);
            }
            
            // Check if there are remaining requests
            if info.remaining_requests > 0 {
                // Update remaining requests
                let new_info = RateLimitInfo {
                    user_id: user_id.to_string(),
                    remaining_requests: info.remaining_requests - 1,
                    reset_at: info.reset_at,
                };
                store.insert(user_id.to_string(), new_info);
                return Ok(true);
            }
            
            // Rate limit exceeded
            return Ok(false);
        }
        
        // First request for this user
        let new_info = RateLimitInfo {
            user_id: user_id.to_string(),
            remaining_requests: self.config.max_requests - 1,
            reset_at: now + Duration::seconds(self.config.window_seconds as i64),
        };
        store.insert(user_id.to_string(), new_info);
        
        Ok(true)
    }
    
    pub async fn reset(&self, user_id: &str) -> Result<()> {
        let mut store = self.store.lock().unwrap();
        let now = Utc::now();
        
        // Set up the user with a fresh rate limit
        let new_info = RateLimitInfo {
            user_id: user_id.to_string(),
            remaining_requests: self.config.max_requests,  // Full quota
            reset_at: now + Duration::seconds(self.config.window_seconds as i64),
        };
        
        store.insert(user_id.to_string(), new_info);
        Ok(())
    }
    
    pub async fn get_limit_info(&self, user_id: &str) -> Option<RateLimitInfo> {
        let store = self.store.lock().unwrap();
        store.get(user_id).cloned()
    }
}