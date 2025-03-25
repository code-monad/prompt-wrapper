use axum::{
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod handlers;
mod models;
mod openrouter;
mod preset;
mod rate_limiter;
mod storage;
pub mod languages;

use crate::config::{Config, StorageType, TEST_USER_ID};
use crate::models::{Saying, SayingSource};
use crate::openrouter::OpenRouterClient;
use crate::preset::Presets;
use crate::rate_limiter::RateLimiter;
use crate::storage::Storage;

// Application state that will be shared between handlers
pub struct AppState {
    pub config: Config,
    pub openrouter: OpenRouterClient,
    pub rate_limiter: RateLimiter,
    pub storage: Storage,
    pub presets: Presets,
}

// Initialize a test user with predefined data (debug mode only)
#[cfg(debug_assertions)]
async fn initialize_test_user(app_state: &Arc<AppState>) -> anyhow::Result<()> {
    tracing::info!("Initializing test user with ID: {}", TEST_USER_ID);
    
    // Initialize rate limit for test user (uses the normal rate limit config)
    // Note: We use reset() which gives the user their full quota, but follows normal rules
    app_state.rate_limiter.reset(TEST_USER_ID).await?;
    
    // Don't pre-populate any sayings - let them be generated dynamically
    // Don't pre-select a preset - let it be selected dynamically
    
    tracing::info!("Test user initialized with empty state (fully dynamic workflow)");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let config = Config::from_env();
    
    // Ensure data directory exists for Sled if needed
    if let StorageType::Sled = config.storage.type_ {
        let path = Path::new(&config.storage.connection_string);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tracing::info!("Creating data directory: {:?}", parent);
                fs::create_dir_all(parent)?;
            }
        }
    }

    // Load presets
    let presets_path = &config.presets.file_path;
    let presets = Presets::from_file(presets_path)?;

    // Initialize services
    let openrouter_client = OpenRouterClient::new(config.openrouter.clone());
    let rate_limiter = RateLimiter::new(config.rate_limit.clone());
    let storage = Storage::new(config.storage.clone());
    
    // Create and share application state
    let app_state = Arc::new(AppState {
        config: config.clone(),
        openrouter: openrouter_client,
        rate_limiter,
        storage,
        presets,
    });
    
    // Initialize test user in debug mode
    #[cfg(debug_assertions)]
    {
        if let Err(e) = initialize_test_user(&app_state).await {
            tracing::warn!("Failed to initialize test user: {}", e);
        }
    }

    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Define routes
    let app = Router::new()
        // Sayings resource
        .route("/sayings", get(handlers::get_sayings).post(handlers::create_saying))
        .route("/sayings/latest", get(handlers::get_latest_saying))
        
        // User status resource
        .route("/users/:user_id/status", get(handlers::get_user_status))
        
        // Presets resource
        .route("/presets", get(handlers::get_presets))
        .route("/presets/:preset_id", get(handlers::get_preset))
        
        // Languages resource
        .route("/languages", get(handlers::get_languages))
        .route("/languages/:language_id", get(handlers::get_language))
        
        .layer(cors)
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid socket address");
    tracing::info!("listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
