use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub button_text: String,
    pub loading_text: String,
    pub instruction_text: String,
    pub system_prompt: String,
    pub user_prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSelection {
    pub preset: Preset,
    pub selected_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presets {
    presets: Vec<Preset>,
    // Map of user_id -> currently selected preset
    selections: Arc<Mutex<std::collections::HashMap<String, PresetSelection>>>,
}

impl Presets {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read presets file: {:?}", path.as_ref()))?;
        
        let presets: Vec<Preset> = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML in presets file: {:?}", path.as_ref()))?;
        
        // Validate presets
        for preset in &presets {
            if preset.id.is_empty() || preset.name.is_empty() || preset.system_prompt.is_empty() || preset.user_prompts.is_empty() {
                return Err(anyhow::anyhow!("Invalid preset in file: {:?}", path.as_ref()));
            }
        }
        
        tracing::info!("Loaded {} presets from {:?}", presets.len(), path.as_ref());
        
        Ok(Self {
            presets,
            selections: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }
    
    pub fn get_or_select_preset(&self, user_id: &str, reset_at: DateTime<Utc>) -> Result<Preset> {
        let mut selections = self.selections.lock().unwrap();
        
        // Check if user already has a selected preset and if it's still valid
        if let Some(selection) = selections.get(user_id) {
            if selection.expires_at > Utc::now() {
                return Ok(selection.preset.clone());
            }
        }
        
        // Select a new random preset
        let preset = self.random_preset()?;
        
        // Store the selection
        selections.insert(user_id.to_string(), PresetSelection {
            preset: preset.clone(),
            selected_at: Utc::now(),
            expires_at: reset_at,
        });
        
        Ok(preset)
    }
    
    pub fn random_preset(&self) -> Result<Preset> {
        let mut rng = rand::thread_rng();
        
        self.presets
            .choose(&mut rng)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No presets available"))
    }
    
    pub fn get_preset_by_id(&self, id: &str) -> Option<Preset> {
        self.presets.iter().find(|p| p.id == id).cloned()
    }
    
    pub fn get_all_presets(&self) -> Vec<Preset> {
        self.presets.clone()
    }
    
    pub fn random_user_prompt(&self, preset_id: &str) -> Result<String> {
        let preset = self.get_preset_by_id(preset_id)
            .ok_or_else(|| anyhow::anyhow!("Preset not found: {}", preset_id))?;
        
        let mut rng = rand::thread_rng();
        
        preset.user_prompts
            .choose(&mut rng)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No user prompts available for preset: {}", preset_id))
    }
    
    pub fn get_default_preset(&self) -> Result<Preset> {
        // First try to find a preset with ID "oracle" (matching the TypeScript default)
        if let Some(preset) = self.get_preset_by_id("oracle") {
            return Ok(preset);
        }
        
        // If not found, return the first preset
        self.presets.first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No presets available"))
    }
}