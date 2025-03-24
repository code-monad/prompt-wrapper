use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::config::{StorageConfig, StorageType};
use crate::models::{Saying, SayingSource};

pub struct Storage {
    inner: MemoryStorage,
}

impl Storage {
    pub fn new(config: StorageConfig) -> Self {
        // For now, we only implement memory storage
        // In a real app, we'd use the config to determine which storage to use
        Self {
            inner: MemoryStorage::new(),
        }
    }

    pub async fn save_saying(&self, user_id: &str, saying: Saying) -> Result<Saying> {
        self.inner.save_saying(user_id, saying)
    }

    pub async fn get_last_saying(&self, user_id: &str) -> Result<Option<Saying>> {
        self.inner.get_last_saying(user_id)
    }

    pub async fn get_sayings(&self, user_id: &str, limit: usize) -> Result<Vec<Saying>> {
        self.inner.get_sayings(user_id, limit)
    }
}

#[derive(Clone)]
struct MemoryStorage {
    // Map of user_id -> list of sayings
    sayings: Arc<Mutex<HashMap<String, Vec<Saying>>>>,
}

impl MemoryStorage {
    fn new() -> Self {
        Self {
            sayings: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn save_saying(&self, user_id: &str, saying: Saying) -> Result<Saying> {
        let mut sayings_map = self.sayings.lock().unwrap();
        
        // Get or create the user's saying list
        let user_sayings = sayings_map.entry(user_id.to_string()).or_insert_with(Vec::new);
        
        // Add the new saying
        user_sayings.push(saying.clone());
        
        // Sort by created_at date (newest first)
        user_sayings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(saying)
    }

    fn get_last_saying(&self, user_id: &str) -> Result<Option<Saying>> {
        let sayings_map = self.sayings.lock().unwrap();
        
        // Get user's sayings if they exist
        if let Some(user_sayings) = sayings_map.get(user_id) {
            if !user_sayings.is_empty() {
                // Return the first saying (newest one due to sorting)
                return Ok(Some(user_sayings[0].clone()));
            }
        }
        
        Ok(None)
    }

    fn get_sayings(&self, user_id: &str, limit: usize) -> Result<Vec<Saying>> {
        let sayings_map = self.sayings.lock().unwrap();
        
        // Get user's sayings if they exist
        if let Some(user_sayings) = sayings_map.get(user_id) {
            let mut result = user_sayings.clone();
            
            if result.len() > limit {
                result.truncate(limit);
            }
            
            return Ok(result);
        }
        
        Ok(Vec::new())
    }
} 