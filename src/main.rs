use axum::{
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod handlers;
mod models;
mod openrouter;
mod rate_limiter;
mod storage;

use crate::config::Config;
use crate::openrouter::OpenRouterClient;
use crate::rate_limiter::RateLimiter;
use crate::storage::Storage;

// Application state that will be shared between handlers
pub struct AppState {
    pub config: Config,
    pub openrouter: OpenRouterClient,
    pub rate_limiter: RateLimiter,
    pub storage: Storage,
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
    });

    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Define routes
    let app = Router::new()
        .route("/saying", post(handlers::create_saying)) // POST to get new saying
        .route("/status", get(handlers::get_status))     // GET to check status
        .layer(cors)
        .with_state(app_state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));
    tracing::info!("listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
