use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::extract::State;
use dotenvy::dotenv;
use reqwest::Client;
use sqlx::SqlitePool;

use crate::{config::Config, encryption::EncryptionProvider};

mod assets;
mod config;
mod db;
mod encryption;
mod error;
mod handler;
mod route;
mod search;
mod trace;

/// Shared application state accessible across all request handlers.
pub struct AppState {
    /// Encryption provider for password hashing and token generation.
    pub encryption: EncryptionProvider,
    /// Database connection pool for `SQLite`.
    pub pool: SqlitePool,
    /// Shared HTTP client for external requests.
    pub http_client: Client,
}

/// Type alias for extracting the application state in request handlers.
pub type ApiState = State<Arc<AppState>>;

/// Main entry point for the application.
///
/// Initializes the application by:
/// 1. Loading environment variables from `.env` file
/// 2. Initializing configuration
/// 3. Establishing database connection
/// 4. Setting up encryption provider
/// 5. Starting the web server
///
/// # Errors
///
/// Returns an error if:
/// - Database connection fails
/// - Server fails to bind to the configured address
/// - Server encounters an unrecoverable error during operation
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from the .env file
    dotenv().ok();
    let config = Config::try_init()?;
    let pool = db::connect(&config.database_url).await?;
    let encryption = EncryptionProvider::new(config.root_key);

    // Create shared HTTP client with optimized settings for title fetching
    let http_client = Client::builder()
        .timeout(Duration::from_millis(1000))
        .connect_timeout(Duration::from_millis(500))
        .user_agent("PagePouch/1.0")
        .build()?;

    let app_state = Arc::new(AppState {
        encryption,
        pool,
        http_client,
    });

    route::serve(app_state).await?;

    Ok(())
}
