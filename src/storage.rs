use anyhow::{Result, Context};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::config::{StorageConfig, StorageType};
use crate::models::{Saying, SayingSource, CacheKey};

pub struct Storage {
    inner: StorageImpl,
}

enum StorageImpl {
    Memory(MemoryStorage),
    Sled(SledStorage),
}

impl Storage {
    pub fn new(config: StorageConfig) -> Self {
        let inner = match config.type_ {
            StorageType::Memory => StorageImpl::Memory(MemoryStorage::new()),
            StorageType::SQLite => {
                // Fallback to memory for now
                tracing::warn!("SQLite storage not implemented yet, using memory storage instead");
                StorageImpl::Memory(MemoryStorage::new())
            }
            StorageType::Redis => {
                // Fallback to memory for now
                tracing::warn!("Redis storage not implemented yet, using memory storage instead");
                StorageImpl::Memory(MemoryStorage::new())
            }
            StorageType::Sled => {
                match SledStorage::new(&config.connection_string) {
                    Ok(storage) => StorageImpl::Sled(storage),
                    Err(e) => {
                        tracing::error!("Failed to initialize Sled storage: {}", e);
                        tracing::warn!("Falling back to memory storage");
                        StorageImpl::Memory(MemoryStorage::new())
                    }
                }
            }
        };

        Self { inner }
    }

    pub async fn save_saying(&self, user_id: &str, saying: Saying) -> Result<Saying> {
        match &self.inner {
            StorageImpl::Memory(storage) => storage.save_saying(user_id, saying),
            StorageImpl::Sled(storage) => storage.save_saying(user_id, saying),
        }
    }

    pub async fn get_last_saying(&self, user_id: &str) -> Result<Option<Saying>> {
        match &self.inner {
            StorageImpl::Memory(storage) => storage.get_last_saying(user_id),
            StorageImpl::Sled(storage) => storage.get_last_saying(user_id),
        }
    }

    pub async fn get_sayings(&self, user_id: &str, limit: usize) -> Result<Vec<Saying>> {
        match &self.inner {
            StorageImpl::Memory(storage) => storage.get_sayings(user_id, limit),
            StorageImpl::Sled(storage) => storage.get_sayings(user_id, limit),
        }
    }

    // Find a saying that matches a prompt and preset_id
    pub async fn find_cached_saying(&self, prompt: &str, preset_id: Option<&str>) -> Result<Option<Saying>> {
        match &self.inner {
            StorageImpl::Memory(storage) => storage.find_cached_saying(prompt, preset_id),
            StorageImpl::Sled(storage) => storage.find_cached_saying(prompt, preset_id),
        }
    }
    
    // Gets any cached sayings from any user (useful for serving during rate-limiting)
    pub async fn get_any_cached_sayings(&self, limit: usize) -> Result<Vec<Saying>> {
        match &self.inner {
            StorageImpl::Memory(storage) => storage.get_any_cached_sayings(limit),
            StorageImpl::Sled(storage) => storage.get_any_cached_sayings(limit),
        }
    }
}

#[derive(Clone)]
struct MemoryStorage {
    // Map of user_id -> list of sayings
    sayings: Arc<Mutex<HashMap<String, Vec<Saying>>>>,
    // Global cache by prompt + preset
    global_cache: Arc<Mutex<HashMap<CacheKey, Saying>>>,
}

impl MemoryStorage {
    fn new() -> Self {
        Self {
            sayings: Arc::new(Mutex::new(HashMap::new())),
            global_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn save_saying(&self, user_id: &str, saying: Saying) -> Result<Saying> {
        // Add to user's sayings
        let mut sayings_map = self.sayings.lock().unwrap();
        
        // Get or create the user's saying list
        let user_sayings = sayings_map.entry(user_id.to_string()).or_insert_with(Vec::new);
        
        // Add the new saying
        let saying_to_save = saying.clone();
        user_sayings.push(saying_to_save.clone());
        
        // Sort by created_at date (newest first)
        user_sayings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Add to global cache if it's not an LLM source (we only cache non-LLM entries)
        if !matches!(saying.source, SayingSource::LLM) {
            let cache_key = CacheKey::from_saying(&saying);
            let mut global_cache = self.global_cache.lock().unwrap();
            global_cache.insert(cache_key, saying.clone());
        }
        
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

    fn find_cached_saying(&self, prompt: &str, preset_id: Option<&str>) -> Result<Option<Saying>> {
        // First check the global cache for direct match
        let cache_key = CacheKey::new(
            preset_id.map(|id| id.to_string()), 
            prompt.to_string()
        );
        
        let global_cache = self.global_cache.lock().unwrap();
        if let Some(cached) = global_cache.get(&cache_key) {
            // We found a direct match in the global cache
            return Ok(Some(cached.clone()));
        }

        // Fall back to checking all user sayings
        let sayings_map = self.sayings.lock().unwrap();
        
        // Search through all users' sayings to find a matching prompt and preset
        for user_sayings in sayings_map.values() {
            for saying in user_sayings {
                if saying.prompt == prompt && 
                   saying.preset_id.as_deref() == preset_id && 
                   !matches!(saying.source, SayingSource::LLM) {
                    // Found a match from cache or database
                    return Ok(Some(saying.clone()));
                }
            }
        }
        
        // No match found
        Ok(None)
    }

    fn get_any_cached_sayings(&self, limit: usize) -> Result<Vec<Saying>> {
        // First try to get sayings from the global cache
        let global_cache = self.global_cache.lock().unwrap();
        let mut all_cached_sayings: Vec<Saying> = global_cache.values().cloned().collect();
        
        // If we don't have enough, fall back to the per-user sayings
        if all_cached_sayings.len() < limit {
            let sayings_map = self.sayings.lock().unwrap();
            
            // Collect sayings from all users, preferring non-LLM sources
            for user_sayings in sayings_map.values() {
                for saying in user_sayings {
                    if !matches!(saying.source, SayingSource::LLM) {
                        // Check if we already have this saying in our result (from global cache)
                        let is_duplicate = all_cached_sayings.iter().any(|s| 
                            s.prompt == saying.prompt && s.preset_id == saying.preset_id
                        );
                        
                        if !is_duplicate {
                            all_cached_sayings.push(saying.clone());
                        }
                    }
                }
            }
            
            // If we still don't have enough, include LLM sources as a fallback
            if all_cached_sayings.len() < limit {
                for user_sayings in sayings_map.values() {
                    for saying in user_sayings {
                        if matches!(saying.source, SayingSource::LLM) {
                            let is_duplicate = all_cached_sayings.iter().any(|s| 
                                s.prompt == saying.prompt && s.preset_id == saying.preset_id
                            );
                            
                            if !is_duplicate {
                                all_cached_sayings.push(saying.clone());
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by date (newest first)
        all_cached_sayings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Limit the results
        if all_cached_sayings.len() > limit {
            all_cached_sayings.truncate(limit);
        }
        
        Ok(all_cached_sayings)
    }
}

struct SledStorage {
    db: sled::Db,
}

impl SledStorage {
    fn new(path: &str) -> Result<Self> {
        let db = sled::open(path).context("Failed to open Sled database")?;
        
        // Ensure the global cache tree exists
        db.open_tree("global_cache").context("Failed to create global cache tree")?;
        
        Ok(Self { db })
    }

    fn save_saying(&self, user_id: &str, saying: Saying) -> Result<Saying> {
        // Get existing sayings for the user
        let mut sayings = self.get_sayings(user_id, usize::MAX)?;
        
        // Add the new saying
        let saying_to_save = saying.clone();
        sayings.push(saying_to_save);
        
        // Sort by created_at date (newest first)
        sayings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Serialize and save user sayings
        let serialized = serde_json::to_vec(&sayings).context("Failed to serialize sayings")?;
        self.db.insert(user_id.as_bytes(), serialized).context("Failed to insert into Sled database")?;
        
        // Add to global cache if it's not an LLM source
        if !matches!(saying.source, SayingSource::LLM) {
            let global_tree = self.db.open_tree("global_cache").context("Failed to open global cache tree")?;
            
            // Create a unique key based on preset + prompt
            let cache_key = CacheKey::from_saying(&saying);
            let key_bytes = serde_json::to_vec(&cache_key).context("Failed to serialize cache key")?;
            
            // Store the saying in the global cache
            let serialized_saying = serde_json::to_vec(&saying).context("Failed to serialize saying for cache")?;
            global_tree.insert(key_bytes, serialized_saying).context("Failed to insert into global cache")?;
        }
        
        Ok(saying)
    }

    fn get_last_saying(&self, user_id: &str) -> Result<Option<Saying>> {
        // Try to get all sayings for the user
        let sayings = self.get_sayings(user_id, 1)?;
        
        // Return the first one if any exist
        if sayings.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sayings[0].clone()))
        }
    }

    fn get_sayings(&self, user_id: &str, limit: usize) -> Result<Vec<Saying>> {
        // Try to get the user's sayings from the database
        match self.db.get(user_id.as_bytes()) {
            Ok(Some(ivec)) => {
                // Deserialize the sayings
                let mut sayings: Vec<Saying> = serde_json::from_slice(&ivec)
                    .context("Failed to deserialize sayings from Sled")?;
                
                // Sort and limit
                sayings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                if sayings.len() > limit {
                    sayings.truncate(limit);
                }
                
                Ok(sayings)
            }
            Ok(None) => Ok(Vec::new()), // No sayings for this user yet
            Err(e) => Err(anyhow::anyhow!("Sled error: {}", e)),
        }
    }

    fn find_cached_saying(&self, prompt: &str, preset_id: Option<&str>) -> Result<Option<Saying>> {
        // First check the global cache for direct match
        let global_tree = self.db.open_tree("global_cache").context("Failed to open global cache tree")?;
        
        let cache_key = CacheKey::new(
            preset_id.map(|id| id.to_string()), 
            prompt.to_string()
        );
        
        let key_bytes = serde_json::to_vec(&cache_key).context("Failed to serialize cache key")?;
        
        // Check if we have this key in the global cache
        if let Ok(Some(ivec)) = global_tree.get(&key_bytes) {
            let saying: Saying = serde_json::from_slice(&ivec)
                .context("Failed to deserialize saying from global cache")?;
            return Ok(Some(saying));
        }
        
        // Fall back to checking all user sayings
        for result in self.db.iter() {
            let (key, ivec) = result.context("Failed to iterate Sled database")?;
            
            // Skip the global cache tree
            if key.starts_with(b"__") {
                continue;
            }
            
            // Deserialize the sayings
            let sayings: Vec<Saying> = serde_json::from_slice(&ivec)
                .context("Failed to deserialize sayings from Sled")?;
            
            // Look for a matching prompt and preset
            for saying in sayings {
                if saying.prompt == prompt && 
                   saying.preset_id.as_deref() == preset_id && 
                   !matches!(saying.source, SayingSource::LLM) {
                    // Found a match from cache or database
                    return Ok(Some(saying));
                }
            }
        }
        
        // No match found
        Ok(None)
    }

    fn get_any_cached_sayings(&self, limit: usize) -> Result<Vec<Saying>> {
        let mut all_cached_sayings = Vec::new();
        
        // First try the global cache
        let global_tree = self.db.open_tree("global_cache").context("Failed to open global cache tree")?;
        
        for result in global_tree.iter() {
            let (_, ivec) = result.context("Failed to iterate global cache")?;
            
            let saying: Saying = serde_json::from_slice(&ivec)
                .context("Failed to deserialize saying from global cache")?;
            
            all_cached_sayings.push(saying);
            
            if all_cached_sayings.len() >= limit {
                // Sort by date (newest first) and return
                all_cached_sayings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                return Ok(all_cached_sayings);
            }
        }
        
        // If we don't have enough from global cache, check user sayings
        let mut seen_keys = HashSet::new();
        
        // Add all non-LLM sayings to our collection
        for result in self.db.iter() {
            let (key, ivec) = result.context("Failed to iterate Sled database")?;
            
            // Skip the global cache tree and other internal trees
            if key.starts_with(b"__") {
                continue;
            }
            
            // Deserialize the sayings
            let sayings: Vec<Saying> = serde_json::from_slice(&ivec)
                .context("Failed to deserialize sayings from Sled")?;
            
            for saying in &sayings {
                if !matches!(saying.source, SayingSource::LLM) {
                    // Create a cache key to track duplicates
                    let cache_key = CacheKey::from_saying(saying);
                    
                    if !seen_keys.contains(&cache_key) {
                        seen_keys.insert(cache_key);
                        all_cached_sayings.push(saying.clone());
                        
                        if all_cached_sayings.len() >= limit {
                            break;
                        }
                    }
                }
            }
            
            if all_cached_sayings.len() >= limit {
                break;
            }
        }
        
        // If we still don't have enough, include LLM sources as a fallback
        if all_cached_sayings.len() < limit {
            for result in self.db.iter() {
                let (key, ivec) = result.context("Failed to iterate Sled database")?;
                
                // Skip the global cache tree and other internal trees
                if key.starts_with(b"__") {
                    continue;
                }
                
                // Deserialize the sayings
                let sayings: Vec<Saying> = serde_json::from_slice(&ivec)
                    .context("Failed to deserialize sayings from Sled")?;
                
                for saying in &sayings {
                    if matches!(saying.source, SayingSource::LLM) {
                        // Create a cache key to track duplicates
                        let cache_key = CacheKey::from_saying(saying);
                        
                        if !seen_keys.contains(&cache_key) {
                            seen_keys.insert(cache_key);
                            all_cached_sayings.push(saying.clone());
                            
                            if all_cached_sayings.len() >= limit {
                                break;
                            }
                        }
                    }
                }
                
                if all_cached_sayings.len() >= limit {
                    break;
                }
            }
        }
        
        // Sort by date (newest first)
        all_cached_sayings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(all_cached_sayings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_memory_storage_find_cached_saying() {
        // Create memory storage
        let storage = MemoryStorage::new();
        
        // Create test sayings with different sources
        let user_id = "test_user";
        let prompt = "test prompt";
        let preset_id = Some("test_preset".to_string());
        
        let llm_saying = Saying {
            id: Uuid::new_v4().to_string(),
            content: "LLM generated content".to_string(),
            prompt: prompt.to_string(),
            created_at: Utc::now(),
            source: SayingSource::LLM,
            preset_id: preset_id.clone(),
        };
        
        let cached_saying = Saying {
            id: Uuid::new_v4().to_string(),
            content: "Cached content".to_string(),
            prompt: prompt.to_string(),
            created_at: Utc::now(),
            source: SayingSource::Cache,
            preset_id: preset_id.clone(),
        };
        
        // Save sayings
        storage.save_saying(user_id, llm_saying.clone()).unwrap();
        storage.save_saying(user_id, cached_saying.clone()).unwrap();
        
        // Test finding cached saying
        let result = storage.find_cached_saying(prompt, preset_id.as_deref()).unwrap();
        
        // Should find cached_saying, not llm_saying
        assert!(result.is_some());
        let found = result.unwrap();
        assert_eq!(found.content, cached_saying.content);
        assert!(matches!(found.source, SayingSource::Cache));
        
        // Test with non-existent prompt
        let no_result = storage.find_cached_saying("nonexistent", preset_id.as_deref()).unwrap();
        assert!(no_result.is_none());
    }

    #[test]
    fn test_sled_storage_find_cached_saying() {
        // Create temp directory for test
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test-sled-db");
        
        // Create sled storage
        let storage = SledStorage::new(db_path.to_str().unwrap()).unwrap();
        
        // Create test sayings with different sources
        let user_id = "test_user";
        let prompt = "test prompt";
        let preset_id = Some("test_preset".to_string());
        
        let llm_saying = Saying {
            id: Uuid::new_v4().to_string(),
            content: "LLM generated content".to_string(),
            prompt: prompt.to_string(),
            created_at: Utc::now(),
            source: SayingSource::LLM,
            preset_id: preset_id.clone(),
        };
        
        let cached_saying = Saying {
            id: Uuid::new_v4().to_string(),
            content: "Cached content".to_string(),
            prompt: prompt.to_string(),
            created_at: Utc::now(),
            source: SayingSource::Cache,
            preset_id: preset_id.clone(),
        };
        
        // Save sayings
        storage.save_saying(user_id, llm_saying.clone()).unwrap();
        storage.save_saying(user_id, cached_saying.clone()).unwrap();
        
        // Test finding cached saying
        let result = storage.find_cached_saying(prompt, preset_id.as_deref()).unwrap();
        
        // Should find cached_saying, not llm_saying
        assert!(result.is_some());
        let found = result.unwrap();
        assert_eq!(found.content, cached_saying.content);
        assert!(matches!(found.source, SayingSource::Cache));
        
        // Test with non-existent prompt
        let no_result = storage.find_cached_saying("nonexistent", preset_id.as_deref()).unwrap();
        assert!(no_result.is_none());
    }
}