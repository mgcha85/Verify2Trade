mod data;
mod indicators;
mod engine;
mod strategy;
mod charting;
mod api;
mod ai;
mod settings;

use axum::{
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::{Arc, Mutex}, collections::HashMap};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::sync::broadcast;
use crate::{data::DataLoader, api::AppState, ai::AIClient, settings::Settings};
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok(); 

    let settings = Settings::new().expect("Failed to load configuration");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let ai_client = AIClient::new(
        env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
        env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3-vl:latest".to_string()),
        env::var("REMOTE_LLM_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        env::var("REMOTE_LLM_API_KEY").unwrap_or_else(|_| "".to_string()),
        env::var("REMOTE_LLM_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
    );

    let (tx, _rx) = broadcast::channel(100);
    
    let app_state = AppState {
        data_loader: Arc::new(DataLoader::new(&settings.backtest.data_path)), 
        backtests: Arc::new(Mutex::new(HashMap::new())),
        progress_tx: tx,
        ai_client: Arc::new(ai_client),
        settings: Arc::new(settings.clone()),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/backtest/run", post(api::run_backtest))
        .route("/api/backtest/progress/{id}", get(api::get_progress_sse))
        .route("/api/backtest/result/{id}", get(api::get_result))
        .route("/api/data/symbols", get(api::list_symbols))
        .with_state(app_state)
        .layer(tower_http::cors::CorsLayer::permissive());

    let port = env::var("API_PORT").unwrap_or_else(|_| "3000".to_string()).parse().unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
