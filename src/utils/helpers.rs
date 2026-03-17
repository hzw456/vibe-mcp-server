use crate::models::TaskStatus;
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use std::sync::Arc;

pub fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn validate_status(status: &str) -> Option<TaskStatus> {
    match status.to_lowercase().as_str() {
        "armed" => Some(TaskStatus::Armed),
        "running" => Some(TaskStatus::Running),
        "completed" => Some(TaskStatus::Completed),
        "error" => Some(TaskStatus::Error),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() >= 5
}

pub async fn authenticate_jwt(
    headers: &HeaderMap,
    state: &Arc<crate::AppState>,
) -> Result<String, StatusCode> {
    use crate::services::auth_service::AuthService;

    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                match AuthService::decode_jwt_token(token, &state.config) {
                    Ok(claims) => return Ok(claims.sub),
                    Err(_) => return Err(StatusCode::UNAUTHORIZED),
                }
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

pub async fn authenticate(
    headers: &HeaderMap,
    api_key: &str,
    state: &Arc<crate::AppState>,
) -> Result<String, StatusCode> {
    use crate::services::auth_service::AuthService;

    // Try JWT first
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                match AuthService::decode_jwt_token(token, &state.config) {
                    Ok(claims) => return Ok(claims.sub),
                    Err(_) => return Err(StatusCode::UNAUTHORIZED),
                }
            }
        }
    }

    // Fallback to API Key
    if let Some(key) = headers.get("x-api-key") {
        if key.to_str().map_or(false, |s| s == api_key) {
            return Ok("api_key_user".to_string());
        }
    }
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") && &auth_str[7..] == api_key {
                return Ok("api_key_user".to_string());
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}
