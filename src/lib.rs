pub mod config;
pub mod handlers;
pub mod models;
pub mod services;
pub mod utils;

pub use config::Config;
pub use models::{Claims, Task, TaskStageHistory, TaskStatus, User, VerificationCode};
pub use services::{AuthService, TaskService, UserService};

use axum::{
    async_trait,
    extract::rejection::JsonRejection,
    extract::{FromRequest, Request},
    http::StatusCode,
    routing::{get, post},
    Json, RequestExt, Router,
};
use sqlx::mysql::MySqlPoolOptions;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub task_service: TaskService,
    pub user_service: UserService,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let db_url = config.database_url.clone();
        if !db_url.is_empty() {
            let db_url_for_probe = db_url.clone();
            tokio::spawn(async move {
                match MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url_for_probe)
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Database probe succeeded during startup");
                        pool.close().await;
                    }
                    Err(error) => {
                        tracing::warn!("Database probe failed during startup: {}", error);
                    }
                }
            });
        } else {
            tracing::warn!(
                "DATABASE_URL is not set; health checks will work, database-backed APIs will not"
            );
        }

        let task_service = TaskService::new(db_url.clone());
        let user_service = UserService::new(db_url);

        Ok(Self {
            config,
            task_service,
            user_service,
        })
    }
}

pub struct ApiJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<(), Rejection = JsonRejection>,
    T: 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        match req.extract::<Json<T>, _>().await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                let status = if rejection.status() == StatusCode::UNPROCESSABLE_ENTITY {
                    StatusCode::BAD_REQUEST
                } else {
                    rejection.status()
                };

                Err((status, rejection.body_text()))
            }
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
        .route(
            "/api/auth/send-verification",
            post(handlers::send_verification),
        )
        .route("/api/auth/verify", post(handlers::verify_code))
        .route("/api/sync/task", post(handlers::sync_task))
        // Task endpoints (require JWT)
        .route("/api/status", get(handlers::get_status))
        .route("/api/history", get(handlers::get_history))
        .route(
            "/api/task/:task_id/stages",
            get(handlers::get_task_stage_history),
        )
        .route("/api/task/update_state", post(handlers::update_task_state))
        .route("/api/task/start", post(handlers::start_task))
        .route(
            "/api/task/update_progress",
            post(handlers::update_task_progress),
        )
        .route("/api/task/delete", post(handlers::delete_task))
        .route("/api/reset", post(handlers::reset_tasks))
        // MCP endpoint
        .route("/mcp", post(handlers::mcp_handler))
        // CORS
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        // State
        .with_state(state_ref)
}
