use crate::services::auth_service::AuthService;
use crate::services::user_service::{LoginRequest, RegisterRequest, SendVerificationRequest, VerifyCodeRequest};
use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !crate::utils::helpers::validate_email(&req.email) {
        return Ok(Json(json!({ "success": false, "error": "Invalid email format" })));
    }
    
    if req.password.len() < 6 {
        return Ok(Json(json!({ "success": false, "error": "Password must be at least 6 characters" })));
    }
    
    if state.user_service.find_user_by_email(&req.email).is_some() {
        return Ok(Json(json!({ "success": false, "error": "Email already registered" })));
    }
    
    let password_hash = AuthService::hash_password(&req.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let user = state.user_service.create_user(&req.email, &password_hash);
    
    Ok(Json(json!({ 
        "success": true, 
        "message": "Registration successful. Please verify your email.",
        "user_id": user.id 
    })))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user = state.user_service.find_user_by_email(&req.email)
        .ok_or_else(|| StatusCode::UNAUTHORIZED)?;
    
    let password_valid = AuthService::verify_password(&req.password, &user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if !password_valid {
        return Ok(Json(json!({ "success": false, "error": "Invalid credentials" })));
    }
    
    let token = AuthService::create_jwt_token(&user, &state.config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({
        "success": true,
        "token": token,
        "user": {
            "id": user.id,
            "email": user.email,
            "is_verified": user.is_verified
        }
    })))
}

pub async fn send_verification(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendVerificationRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !crate::utils::helpers::validate_email(&req.email) {
        return Ok(Json(json!({ "success": false, "error": "Invalid email format" })));
    }
    
    if state.user_service.find_user_by_email(&req.email).is_none() {
        return Ok(Json(json!({ "success": false, "error": "User not found" })));
    }
    
    let code = AuthService::generate_verification_code();
    state.user_service.save_verification_code(&req.email, &code, 10);
    
    log::info!("Verification code for {}: {}", req.email, code);
    
    Ok(Json(json!({ 
        "success": true, 
        "message": "Verification code sent",
        "code": code
    })))
}

pub async fn verify_code(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyCodeRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !crate::utils::helpers::validate_email(&req.email) {
        return Ok(Json(json!({ "success": false, "error": "Invalid email format" })));
    }
    
    if state.user_service.verify_code(&req.email, &req.code) {
        state.user_service.set_user_verified(&req.email);
        Ok(Json(json!({ "success": true, "message": "Email verified successfully" })))
    } else {
        Ok(Json(json!({ "success": false, "error": "Invalid or expired verification code" })))
    }
}
