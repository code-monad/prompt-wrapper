use axum::{
    extract::{Json, State},
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::rate_limiter::RateLimiter;
use crate::openrouter::OpenRouterClient;
use crate::storage::Storage;
use crate::models::{Saying, SayingSource};
use crate::AppState;

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
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub can_query: bool,
    pub remaining_requests: u32,
    pub reset_at: Option<DateTime<Utc>>,
    pub last_saying: Option<SayingResponse>,
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

// Get status including rate limit info and last saying
pub async fn get_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Get user ID from request (e.g., from token or header)
    let user_id = "user123";
    
    // Check rate limit for the user
    let rate_limit_info = match state.rate_limiter.get_limit_info(user_id).await {
        Some(info) => info,
        None => {
            // User has no rate limit info yet, return default values
            return (
                StatusCode::OK,
                Json(StatusResponse {
                    can_query: true,
                    remaining_requests: state.config.rate_limit.max_requests,
                    reset_at: None,
                    last_saying: None,
                }),
            );
        }
    };
    
    // Get the last saying for this user from storage
    let last_saying = match state.storage.get_last_saying(user_id).await {
        Ok(Some(saying)) => Some(SayingResponse::from(saying)),
        _ => None,
    };
    
    let response = StatusResponse {
        can_query: rate_limit_info.remaining_requests > 0,
        remaining_requests: rate_limit_info.remaining_requests,
        reset_at: Some(rate_limit_info.reset_at),
        last_saying,
    };
    
    (StatusCode::OK, Json(response))
}

// Create a new saying with a specific prompt (or default prompt)
pub async fn create_saying(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SayingRequest>,
) -> impl IntoResponse {
    // TODO: Get user ID from request
    let user_id = "user123";
    
    // Check rate limit for the user
    match state.rate_limiter.check(user_id).await {
        Ok(true) => {}, // Can proceed
        Ok(false) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Rate limit exceeded",
                    "message": "You have exceeded the rate limit for this endpoint"
                })),
            );
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "message": "Failed to check rate limit"
                })),
            );
        }
    };
    
    let prompt = payload.prompt.unwrap_or_else(|| "Give me a wise saying".to_string());
    
    // Query the LLM using OpenRouter
    let saying = match state.openrouter.get_saying(&prompt).await {
        Ok(saying) => saying,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to get saying from LLM",
                    "message": e.to_string()
                })),
            );
        }
    };
    
    // Store the saying for this user
    match state.storage.save_saying(user_id, saying.clone()).await {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Failed to save saying: {}", e);
            // Continue even if saving fails
        }
    }
    
    // Return the new saying
    let response = SayingResponse::from(saying);
    (StatusCode::CREATED, Json(json!(response)))
} 