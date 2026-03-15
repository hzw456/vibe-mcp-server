pub mod config;
pub mod models;
pub mod services;
pub mod handlers;
pub mod utils;

pub use config::Config;
pub use models::{Task, TaskStatus, User, VerificationCode, Claims};
pub use services::{AuthService, TaskService, UserService};

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub task_service: TaskService,
    pub user_service: UserService,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let db_url = config.database_url.clone();
        
        let task_service = TaskService::new(db_url.clone());
        let user_service = UserService::new(db_url);

        Self {
            config,
            task_service,
            user_service,
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    let state = Arc::new(state);
    let state_ref = Arc::clone(&state);
    
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        
        // Auth endpoints
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/refresh-api-key", post(handlers::refresh_api_key))
        .route("/api/auth/send-verification", post(handlers::send_verification))
        .route("/api/auth/verify", post(handlers::verify_code))
        .route("/api/sync/task", post(handlers::sync_task))
        
        // Task endpoints (require JWT)
        .route("/api/status", get(handlers::get_status))
        .route("/api/history", get(handlers::get_history))
        .route("/api/task/update_state", post(handlers::update_task_state))
        .route("/api/task/update_progress", post(handlers::update_task_progress))
        .route("/api/task/delete", post(handlers::delete_task))
        .route("/api/reset", post(handlers::reset_tasks))
        
        // MCP endpoint
        .route("/mcp", post(handlers::mcp_handler))
        
        // CORS
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        
        // State
        .with_state(state_ref)
}
