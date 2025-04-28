use axum::{
    extract::{Json, Path, Query, State},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use rand::{self, seq::SliceRandom};
use thiserror::Error;

use crate::models::{Saying, SayingSource};
use crate::preset::Preset;
use crate::config::TEST_USER_ID;
use crate::AppState;
use crate::languages::{Language, get_all_languages, get_language_by_id};

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("OpenRouter API error: {0}")]
    OpenRouterError(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ApiError::AccessDenied(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::OpenRouterError(err) => (StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
        };

        tracing::error!("{}: {}", status, error_message);
        
        let body = Json(json!({
            "error": self.to_string(),
            "message": error_message,
        }));

        (status, body).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SayingResponse {
    pub id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct SayingRequest {
    pub prompt: Option<String>,
    pub preset_id: Option<String>,
    pub language_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserStatusResponse {
    pub user_id: String,
    pub can_query: bool,
    pub remaining_requests: u32,
    pub reset_at: Option<DateTime<Utc>>,
    pub last_saying: Option<SayingResponse>,
    pub selected_preset: Option<PresetResponse>,
}

#[derive(Debug, Serialize)]
pub struct PresetResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub button_text: String,
    pub loading_text: String,
    pub instruction_text: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub user_id: Option<String>,
    pub language_id: Option<String>,
}

// Convert Preset to PresetResponse
impl From<Preset> for PresetResponse {
    fn from(preset: Preset) -> Self {
        Self {
            id: preset.id,
            name: preset.name,
            description: preset.description,
            tags: preset.tags,
            button_text: preset.button_text,
            loading_text: preset.loading_text,
            instruction_text: preset.instruction_text,
        }
    }
}

// Convert from our internal Saying model to the API response
impl From<Saying> for SayingResponse {
    fn from(saying: Saying) -> Self {
        Self {
            id: saying.id,
            content: saying.content,
            created_at: saying.created_at,
            source: String::from(saying.source),
        }
    }
}

// Function to validate if a user is allowed to access the API
fn is_user_allowed(user_id: &str) -> Result<(), ApiError> {
    // In debug mode, allow the test user (but still follow normal workflow)
    #[cfg(debug_assertions)]
    if user_id == TEST_USER_ID {
        tracing::debug!("Test user accessing API in debug mode (follows normal workflow)");
        return Ok(());
    }

    // In release mode, block the test user
    #[cfg(not(debug_assertions))]
    if user_id == TEST_USER_ID {
        tracing::warn!("Blocked test user access attempt in release mode");
        return Err(ApiError::AccessDenied("This user ID is not allowed in production".to_string()));
    }

    // Regular users are always allowed
    Ok(())
}

// GET /sayings - Get all sayings (with optional limit)
pub async fn get_sayings(
    Query(params): Query<SayingsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SayingResponse>>, ApiError> {
    let user_id = params.user_id.unwrap_or_else(|| "default_user".to_string());
    
    // Check if user is allowed
    is_user_allowed(&user_id)?;
    
    let limit = params.limit.unwrap_or(10);
    
    let sayings = state.storage.get_sayings(&user_id, limit).await
        .map_err(|e| ApiError::InternalError(format!("Failed to get sayings: {}", e)))?;
    
    let response = sayings.into_iter()
        .map(SayingResponse::from)
        .collect::<Vec<_>>();
    
    Ok(Json(response))
}

// GET /sayings/latest - Get the latest saying for a user
pub async fn get_latest_saying(
    Query(params): Query<StatusQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SayingResponse>, ApiError> {
    let user_id = params.user_id.unwrap_or_else(|| "default_user".to_string());
    
    // Check if user is allowed
    is_user_allowed(&user_id)?;
    
    let saying = state.storage.get_last_saying(&user_id).await
        .map_err(|e| ApiError::InternalError(format!("Failed to get saying: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User has no saved sayings".to_string()))?;
    
    Ok(Json(SayingResponse::from(saying)))
}

// POST /sayings - Create a new saying
pub async fn create_saying(
    Query(params): Query<StatusQuery>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SayingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = params.user_id.unwrap_or_else(|| "default_user".to_string());
    
    // Get the language ID from the query or the request body, defaulting to English
    let language_id = params.language_id
        .or(payload.language_id.clone())
        .unwrap_or_else(|| crate::languages::DEFAULT_LANGUAGE_ID.to_string());
    
    // First check if user is in cooldown period (rate limited)
    let is_rate_limited = match state.rate_limiter.get_limit_info(&user_id).await {
        Some(info) => info.remaining_requests == 0,
        None => false, // No rate limit info yet, not limited
    };

    // If user is rate limited, try to return a cached saying randomly
    if is_rate_limited {
        tracing::info!("User {} is in cooldown period, attempting to return cached saying", user_id);
        
        // First try to get their own last saying
        let mut potential_saying = state.storage.get_last_saying(&user_id).await.ok().flatten();
        
        // If no personal saying is available, try to get any cached sayings from the system
        if potential_saying.is_none() {
            match state.storage.get_any_cached_sayings(5).await { // Fetch up to 5
                Ok(sayings) if !sayings.is_empty() => {
                    // Select one randomly
                    potential_saying = sayings.choose(&mut rand::thread_rng()).cloned();
                    if potential_saying.is_some() {
                        tracing::debug!("Returning randomly selected cached saying from system during cooldown");
                    } else {
                        tracing::warn!("Failed to select a random saying from the fetched list for user {}", user_id);
                    }
                }
                Ok(_) => {
                    tracing::warn!("No cached sayings available for rate-limited user {}", user_id);
                }
                Err(err) => {
                    tracing::error!("Error fetching cached sayings for rate-limited user {}: {}", user_id, err);
                    // Fall through to return rate limit error
                }
            }
        } else {
            tracing::debug!("Returning user's last saying during cooldown period");
        }

        // If we found a saying (either last or random cached), return it
        if let Some(saying) = potential_saying {
             // Ensure the source is marked as cache
             let cached_saying = Saying {
                source: SayingSource::Cache,
                ..saying
             };
            return Ok((StatusCode::OK, Json(SayingResponse::from(cached_saying))));
        } else {
            // If absolutely no saying could be returned, enforce rate limit
            tracing::warn!("Rate limit exceeded for user {} and no cached saying found.", user_id);
            return Err(ApiError::RateLimited("You have exceeded the rate limit and no cached saying was available.".to_string()));
        }
    }

    // Access check (moved after initial rate limit check)
    is_user_allowed(&user_id)?;
    
    // Resolve prompt selection regardless of rate limiting
    let (system_prompt, user_prompt, preset_id) = match (payload.prompt.clone(), payload.preset_id.clone()) {
        // User provided their own prompt
        (Some(prompt), _) => {
            ("You are a helpful assistant.".to_string(), prompt, None)
        },
        
        // User specified a preset
        (None, Some(preset_id)) => {
            let preset = state.presets.get_preset_by_id(&preset_id)
                .ok_or_else(|| ApiError::BadRequest(format!("Preset not found: {}", preset_id)))?;
            
            let prompt = state.presets.random_user_prompt(&preset_id)
                .map_err(|e| ApiError::BadRequest(format!("Failed to get prompt from preset: {}", e)))?;
            
            (preset.system_prompt, prompt, Some(preset_id))
        },
        
        // No prompt or preset specified, try to use the selected preset for the user
        (None, None) => {
            // Get or initialize rate limit info for the user
            let rate_limit_info = match state.rate_limiter.get_limit_info(&user_id).await {
                Some(info) => info,
                None => {
                    // User has no rate limit info, initialize it first
                    state.rate_limiter.reset(&user_id).await
                        .map_err(|e| ApiError::InternalError(format!("Failed to initialize rate limit: {}", e)))?;
                    
                    // Now get the newly initialized rate limit info
                    state.rate_limiter.get_limit_info(&user_id).await
                        .ok_or_else(|| ApiError::InternalError("Failed to get rate limit info after initialization".to_string()))?
                }
            };
            
            // Get or select a preset for the user
            let preset = state.presets.get_or_select_preset(&user_id, rate_limit_info.reset_at)
                .map_err(|e| ApiError::InternalError(format!("Failed to select preset: {}", e)))?;
            
            let prompt = state.presets.random_user_prompt(&preset.id)
                .map_err(|e| ApiError::InternalError(format!("Failed to get prompt from preset: {}", e)))?;
            
            (preset.system_prompt, prompt, Some(preset.id))
        }
    };

    // Append translation instructions to system_prompt if language is not English
    let system_prompt_with_language = if language_id != crate::languages::DEFAULT_LANGUAGE_ID {
        let translation_prompt = crate::languages::get_translation_prompt(&language_id);
        if !translation_prompt.is_empty() {
            format!("{}\n\n{}", system_prompt, translation_prompt)
        } else {
            system_prompt
        }
    } else {
        system_prompt
    };

    tracing::info!("Processing request for user '{}' with prompt: {} and preset: {:?} in language: {}", 
                   user_id, user_prompt, preset_id, language_id);

    // Check rate limit before proceeding with LLM
    let can_proceed = state.rate_limiter.check(&user_id).await
        .map_err(|e| ApiError::InternalError(format!("Failed to check rate limit: {}", e)))?;
    
    if !can_proceed {
        // This should technically not be reached if the logic above is correct, but kept as safeguard
        tracing::warn!("Rate limit check failed unexpectedly after initial check for user {}", user_id);
        return Err(ApiError::RateLimited("You have exceeded the rate limit for this endpoint".to_string()));
    }
    
    // Rate limit allows proceeding, fetch directly from LLM
    tracing::info!("Rate limit permits, querying LLM for prompt: {} for user {}", user_prompt, user_id);
    let saying = fetch_from_llm(&state, &system_prompt_with_language, &user_prompt, preset_id).await?;
    
    // Store the saying for this user
    if let Err(e) = state.storage.save_saying(&user_id, saying.clone()).await {
        tracing::error!("Failed to save saying for user {}: {}", user_id, e);
        // Continue even if saving fails
    } else {
        tracing::info!("Successfully saved saying for user: {}", user_id);
    }
    
    // Return the new saying
    let response = SayingResponse::from(saying);
    tracing::info!("Returning new saying with ID: {}", response.id);
    
    Ok((StatusCode::CREATED, Json(response)))
}

// Helper function to fetch from LLM
async fn fetch_from_llm(
    state: &Arc<AppState>,
    system_prompt: &str,
    user_prompt: &str,
    preset_id: Option<String>
) -> Result<Saying, ApiError> {
    let saying = state.openrouter.get_saying_with_system(system_prompt, user_prompt).await
        .map_err(|e| {
            tracing::error!("OpenRouter API error: {}", e);
            ApiError::OpenRouterError(e)
        })?;
    
    // Set preset_id if available
    let saying_with_preset = Saying {
        preset_id,
        ..saying
    };
    
    Ok(saying_with_preset)
}

// GET /users/:user_id/status - Get user status
pub async fn get_user_status(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<UserStatusResponse>, ApiError> {
    // Check if user is allowed
    is_user_allowed(&user_id)?;
    
    // Check rate limit for the user
    let rate_limit_info = match state.rate_limiter.get_limit_info(&user_id).await {
        Some(info) => info,
        None => {
            // User has no rate limit info yet, return default values
            // Try to get a default preset
            let selected_preset = state.presets.get_default_preset()
                .map(|preset| Some(PresetResponse::from(preset)))
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to get default preset: {}", e);
                    None
                });
            
            let response = UserStatusResponse {
                user_id: user_id.clone(),
                can_query: true,
                remaining_requests: state.config.rate_limit.max_requests,
                reset_at: None,
                last_saying: None,
                selected_preset,
            };
            
            return Ok(Json(response));
        }
    };
    
    // Get the last saying for this user from storage
    let last_saying = state.storage.get_last_saying(&user_id).await
        .ok()
        .and_then(|result| result.map(SayingResponse::from));
    
    // Get or select a preset for the user if they can query
    let selected_preset = if rate_limit_info.remaining_requests > 0 {
        state.presets.get_or_select_preset(&user_id, rate_limit_info.reset_at)
            .map(|preset| Some(PresetResponse::from(preset)))
            .unwrap_or_else(|e| {
                tracing::error!("Failed to select preset: {}", e);
                None
            })
    } else {
        None
    };
    
    let response = UserStatusResponse {
        user_id: user_id.clone(),
        can_query: rate_limit_info.remaining_requests > 0,
        remaining_requests: rate_limit_info.remaining_requests,
        reset_at: Some(rate_limit_info.reset_at),
        last_saying,
        selected_preset,
    };
    
    Ok(Json(response))
}

// GET /presets - Get all available presets
pub async fn get_presets(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<PresetResponse>> {
    let presets = state.presets.get_all_presets();
    let response = presets.into_iter()
        .map(PresetResponse::from)
        .collect::<Vec<_>>();
    
    Json(response)
}

// GET /presets/:preset_id - Get a specific preset
pub async fn get_preset(
    Path(preset_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PresetResponse>, ApiError> {
    let preset = state.presets.get_preset_by_id(&preset_id)
        .ok_or_else(|| ApiError::NotFound(format!("No preset with ID: {}", preset_id)))?;
    
    Ok(Json(PresetResponse::from(preset)))
}

#[derive(Debug, Deserialize)]
pub struct SayingsQuery {
    pub user_id: Option<String>,
    pub limit: Option<usize>,
}

// GET /languages - Get all available languages
pub async fn get_languages() -> Json<Vec<Language>> {
    let languages = get_all_languages();
    Json(languages)
}

// GET /languages/:language_id - Get a specific language by ID
pub async fn get_language(
    Path(language_id): Path<String>,
) -> Result<Json<Language>, ApiError> {
    let language = get_language_by_id(&language_id);
    Ok(Json(language))
} 