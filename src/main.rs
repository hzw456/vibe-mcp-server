//! Vibe MCP Server - AI Task Status Tracker
//! 基于 MCP 协议的 AI 任务状态跟踪服务

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};

// ============ 配置 ============

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub api_key: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3010,
            api_key: std::env::var("API_KEY").unwrap_or_else(|_| "vibe-mcp-secret-key".to_string()),
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "vibe-jwt-secret-key-change-in-production".to_string()),
            jwt_expiry_hours: std::env::var("JWT_EXPIRY_HOURS").unwrap_or_else(|_| "24".to_string()).parse().unwrap_or(24),
        }
    }
}

// ============ 数据模型 ============

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Armed,
    Running,
    Completed,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub is_focused: bool,
    pub ide: String,
    pub window_title: String,
    pub project_path: Option<String>,
    pub active_file: Option<String>,
    pub status: TaskStatus,
    pub source: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub last_heartbeat: i64,
    pub estimated_duration: Option<i64>,
    pub current_stage: Option<String>,
}

impl Task {
    pub fn new(id: String, user_id: String, name: String, ide: String, window_title: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id,
            user_id,
            name,
            is_focused: false,
            ide,
            window_title,
            project_path: None,
            active_file: None,
            status: TaskStatus::Armed,
            source: "mcp".to_string(),
            start_time: 0,
            end_time: None,
            last_heartbeat: now,
            estimated_duration: None,
            current_stage: None,
        }
    }
}

// ============ 用户模型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: i64,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCode {
    pub code: String,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

// ============ 请求结构 ============

#[derive(Debug, Deserialize, Default)]
pub struct UpdateStateRequest {
    pub task_id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub estimated_duration: Option<i64>,
    #[serde(default)]
    pub current_stage: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateProgressRequest {
    pub task_id: String,
    #[serde(default)]
    pub estimated_duration_ms: Option<i64>,
    #[serde(default)]
    pub current_stage: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ResetRequest {
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteTaskRequest {
    pub task_id: String,
}

// ============ 认证请求 ============

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SendVerificationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyCodeRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenRefreshRequest {
    pub refresh_token: String,
}

// ============ 服务状态 ============

#[derive(Clone)]
pub struct AppState {
    pub tasks: Arc<Mutex<HashMap<String, Task>>>,
    pub users: Arc<Mutex<HashMap<String, User>>>,
    pub verification_codes: Arc<Mutex<HashMap<String, VerificationCode>>>,
    pub config: Config,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            users: Arc::new(Mutex::new(HashMap::new())),
            verification_codes: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
}

// ============ 工具函数 ============

fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn validate_status(status: &str) -> Option<TaskStatus> {
    match status.to_lowercase().as_str() {
        "armed" => Some(TaskStatus::Armed),
        "running" => Some(TaskStatus::Running),
        "completed" => Some(TaskStatus::Completed),
        "error" => Some(TaskStatus::Error),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() >= 5
}

// ============ 密码哈希 ============

fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, 6)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}

// ============ JWT ============

fn create_jwt_token(user: &User, config: &Config) -> Result<String, jsonwebtoken::errors::Error> {
    let expiry = Utc::now()
        .checked_add_signed(Duration::hours(config.jwt_expiry_hours))
        .unwrap()
        .timestamp();
    
    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        exp: expiry,
        iat: Utc::now().timestamp(),
    };
    
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
}

fn decode_jwt_token(token: &str, config: &Config) -> Result<Claims, jsonwebtoken::errors::Error> {
    jsonwebtoken::decode(
        token,
        &jsonwebtoken::DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .map(|data| data.claims)
}

// ============ 验证码 ============

fn generate_verification_code() -> String {
    const CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const CODE_LENGTH: usize = 6;
    
    let mut rng = rand::thread_rng();
    (0..CODE_LENGTH)
        .map(|_| {
            let idx = rand::Rng::gen_range(&mut rng, 0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// ============ 业务逻辑 ============

impl AppState {
    pub fn get_tasks(&self, user_id: Option<&str>) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        let mut tasks_vec: Vec<Task> = if let Some(uid) = user_id {
            tasks.values().filter(|t| &t.user_id == uid).cloned().collect()
        } else {
            tasks.values().cloned().collect()
        };
        tasks_vec.sort_by(|a, b| b.id.cmp(&a.id));
        tasks_vec
    }

    pub fn update_task_status(&self, req: &UpdateStateRequest, user_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
        // 如果任务不存在，创建一个新任务
        let task = tasks
            .entry(req.task_id.clone())
            .or_insert_with(|| {
                Task::new(
                    req.task_id.clone(),
                    user_id.to_string(),
                    req.task_id.clone(),
                    "IDE".to_string(),
                    req.task_id.clone(),
                )
            });
        
        // 验证任务归属
        if &task.user_id != user_id {
            return Err("Task not found".to_string());
        }
        
        if let Some(status_str) = &req.status {
            let new_status = validate_status(status_str)
                .ok_or_else(|| format!("Invalid status: {}", status_str))?;
            
            task.status = new_status;
            
            match new_status {
                TaskStatus::Running => {
                    if task.start_time == 0 {
                        task.start_time = now_millis();
                    }
                }
                TaskStatus::Completed | TaskStatus::Error | TaskStatus::Cancelled => {
                    task.end_time = Some(now_millis());
                    if new_status == TaskStatus::Completed {
                        task.current_stage = Some("__completed__".to_string());
                    }
                }
                TaskStatus::Armed => {
                    task.estimated_duration = None;
                    task.current_stage = None;
                }
            }
        }
        
        if let Some(est) = req.estimated_duration {
            task.estimated_duration = Some(est);
        }
        if let Some(stage) = &req.current_stage {
            task.current_stage = Some(stage.clone());
        }
        
        task.last_heartbeat = now_millis();
        Ok(())
    }

    pub fn update_task_progress(&self, req: &UpdateProgressRequest, user_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
        let task = tasks.get_mut(&req.task_id)
            .ok_or_else(|| format!("Task not found: {}", req.task_id))?;
        
        // 验证任务归属
        if &task.user_id != user_id {
            return Err("Task not found".to_string());
        }
        
        if let Some(est) = req.estimated_duration_ms {
            task.estimated_duration = Some(est);
        }
        if let Some(stage) = &req.current_stage {
            task.current_stage = Some(stage.clone());
        }
        
        task.last_heartbeat = now_millis();
        Ok(())
    }

    pub fn reset_tasks(&self, task_id: Option<String>, user_id: &str) {
        let mut tasks = self.tasks.lock().unwrap();
        match task_id {
            Some(id) => { 
                if let Some(task) = tasks.get(&id) {
                    if task.user_id == user_id {
                        tasks.remove(&id);
                    }
                }
            }
            None => { 
                tasks.retain(|_, t| t.user_id != user_id);
            }
        }
    }

    pub fn delete_task(&self, task_id: &str, user_id: &str) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get(task_id) {
            if task.user_id == user_id {
                return tasks.remove(task_id).is_some();
            }
        }
        false
    }

    pub fn calculate_progress(&self, task: &Task) -> u32 {
        if task.status == TaskStatus::Completed {
            return 100;
        }
        if let Some(estimated) = task.estimated_duration {
            if estimated > 0 && task.start_time > 0 {
                let elapsed = now_millis() - task.start_time;
                return std::cmp::min(((elapsed as f64 / estimated as f64) * 100.0) as u32, 99);
            }
        }
        0
    }

    // ============ 用户相关 ============

    pub fn find_user_by_email(&self, email: &str) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.values().find(|u| u.email == email).cloned()
    }

    pub fn find_user_by_id(&self, id: &str) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.get(id).cloned()
    }

    pub fn create_user(&self, email: &str, password_hash: &str) -> User {
        let mut users = self.users.lock().unwrap();
        let user = User {
            id: generate_id(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            created_at: now_millis(),
            is_verified: false,
        };
        users.insert(user.id.clone(), user.clone());
        user
    }

    pub fn save_verification_code(&self, email: &str, code: &str, expires_minutes: i64) {
        let mut codes = self.verification_codes.lock().unwrap();
        let expires_at = Utc::now()
            .checked_add_signed(Duration::minutes(expires_minutes))
            .unwrap()
            .timestamp_millis();
        codes.insert(email.to_string(), VerificationCode {
            code: code.to_string(),
            expires_at,
        });
    }

    pub fn verify_code(&self, email: &str, code: &str) -> bool {
        let mut codes = self.verification_codes.lock().unwrap();
        if let Some(stored) = codes.get(email) {
            if stored.code == code && stored.expires_at > now_millis() {
                codes.remove(email);
                return true;
            }
        }
        false
    }

    pub fn set_user_verified(&self, email: &str) {
        let mut users = self.users.lock().unwrap();
        for user in users.values_mut() {
            if user.email == email {
                user.is_verified = true;
                break;
            }
        }
    }
}

// ============ 认证中间件 ============

async fn authenticate_jwt(
    headers: &HeaderMap,
    state: &Arc<AppState>,
) -> Result<String, StatusCode> {
    // Try JWT first
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                match decode_jwt_token(token, &state.config) {
                    Ok(claims) => return Ok(claims.sub),
                    Err(_) => return Err(StatusCode::UNAUTHORIZED),
                }
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn authenticate(
    headers: &HeaderMap,
    api_key: &str,
    state: &Arc<AppState>,
) -> Result<String, StatusCode> {
    // Try JWT first
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                match decode_jwt_token(token, &state.config) {
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

// ============ API 处理器 ============

// Health check
async fn health() -> &'static str {
    "OK"
}

// Status endpoint (returns tasks for the authenticated user)
async fn get_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let user_id = authenticate_jwt(&headers, &state).await?;
    let tasks = state.get_tasks(Some(&user_id));
    Ok(Json(json!({ "tasks": tasks, "taskCount": tasks.len() })))
}

// ============ 认证端点 ============

async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Validate email
    if !validate_email(&req.email) {
        return Ok(Json(json!({ "success": false, "error": "Invalid email format" })));
    }
    
    // Validate password
    if req.password.len() < 6 {
        return Ok(Json(json!({ "success": false, "error": "Password must be at least 6 characters" })));
    }
    
    // Check if user exists
    if state.find_user_by_email(&req.email).is_some() {
        return Ok(Json(json!({ "success": false, "error": "Email already registered" })));
    }
    
    // Hash password and create user
    let password_hash = hash_password(&req.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let user = state.create_user(&req.email, &password_hash);
    
    Ok(Json(json!({ 
        "success": true, 
        "message": "Registration successful. Please verify your email.",
        "user_id": user.id 
    })))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Find user
    let user = state.find_user_by_email(&req.email)
        .ok_or_else(|| StatusCode::UNAUTHORIZED)?;
    
    // Verify password
    let password_valid = verify_password(&req.password, &user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if !password_valid {
        return Ok(Json(json!({ "success": false, "error": "Invalid credentials" })));
    }
    
    // Create JWT token
    let token = create_jwt_token(&user, &state.config)
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

async fn send_verification(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendVerificationRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !validate_email(&req.email) {
        return Ok(Json(json!({ "success": false, "error": "Invalid email format" })));
    }
    
    // Check if user exists
    if state.find_user_by_email(&req.email).is_none() {
        return Ok(Json(json!({ "success": false, "error": "User not found" })));
    }
    
    // Generate and save verification code
    let code = generate_verification_code();
    state.save_verification_code(&req.email, &code, 10); // 10 minutes expiry
    
    // TODO: In production, send actual email here
    // For now, log the code
    log::info!("Verification code for {}: {}", req.email, code);
    
    // TEMP: Return code for testing
    Ok(Json(json!({ 
        "success": true, 
        "message": "Verification code sent",
        "code": code  // TEMP for testing
    })))
}

async fn verify_code(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyCodeRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !validate_email(&req.email) {
        return Ok(Json(json!({ "success": false, "error": "Invalid email format" })));
    }
    
    if state.verify_code(&req.email, &req.code) {
        state.set_user_verified(&req.email);
        Ok(Json(json!({ "success": true, "message": "Email verified successfully" })))
    } else {
        Ok(Json(json!({ "success": false, "error": "Invalid or expired verification code" })))
    }
}

// ============ 任务端点 ============

async fn update_task_state(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateStateRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = authenticate_jwt(&headers, &state).await?;
    state.update_task_status(&req, &user_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn update_task_progress(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = authenticate_jwt(&headers, &state).await?;
    state.update_task_progress(&req, &user_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn delete_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<DeleteTaskRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = authenticate_jwt(&headers, &state).await?;
    if state.delete_task(&req.task_id, &user_id) {
        Ok(Json(json!({"status": "ok"})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn reset_tasks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ResetRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = authenticate_jwt(&headers, &state).await?;
    state.reset_tasks(req.task_id, &user_id);
    Ok(Json(json!({"status": "ok"})))
}

// ============ MCP 协议处理器 ============

async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = authenticate_jwt(&headers, &state).await?;
    
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_default();
    let req_id = req.get("id").cloned();
    
    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "vibe-mcp-server", "version": "1.0.0" }
        }),
        "notifications/initialized" => {
            return Ok(Json(json!({ "jsonrpc": "2.0", "result": {}, "id": req_id })));
        }
        "tools/list" => json!({
            "tools": [
                { "name": "list_tasks", "description": "Get all tasks for the authenticated user", 
                  "inputSchema": { "type": "object", "properties": {}, "required": [] } },
                { "name": "update_task_status", "description": "Update task status",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" }, "status": { "type": "string" } }, "required": ["task_id", "status"] } },
                { "name": "update_task_progress", "description": "Update task progress",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" }, "estimated_duration_ms": { "type": "integer" }, "current_stage": { "type": "string" } }, "required": ["task_id"] } }
            ]
        }),
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_default();
            
            match tool_name {
                "list_tasks" => {
                    let tasks = state.get_tasks(Some(&user_id));
                    let list: Vec<Value> = tasks.iter().map(|t| json!({
                        "id": t.id, "name": t.name, "ide": t.ide, "status": format!("{:?}", t.status).to_lowercase(),
                        "progress": state.calculate_progress(t), "source": t.source, "current_stage": t.current_stage
                    })).collect();
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&list).unwrap() }] })
                }
                "update_task_status" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let status = args.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(), status: Some(status.to_string()), 
                        source: Some("mcp".to_string()), ..Default::default()
                    };
                    match state.update_task_status(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Task {} -> {}", task_id, status) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "update_task_progress" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let req = UpdateProgressRequest {
                        task_id: task_id.to_string(),
                        estimated_duration_ms: args.get("estimated_duration_ms").and_then(|v| v.as_i64()),
                        current_stage: args.get("current_stage").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    };
                    match state.update_task_progress(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Progress updated for {}", task_id) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                _ => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32601, "message": format!("Unknown: {}", tool_name) }, "id": req_id })))
            }
        }
        _ => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32601, "message": format!("Method: {}", method) }, "id": req_id })))
    };
    
    Ok(Json(json!({ "jsonrpc": "2.0", "result": result, "id": req_id })))
}

// ============ 路由配置 ============

fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health))
        
        // Auth endpoints
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/send-verification", post(send_verification))
        .route("/api/auth/verify", post(verify_code))
        
        // Task endpoints (require JWT)
        .route("/api/status", get(get_status))
        .route("/api/task/update_state", post(update_task_state))
        .route("/api/task/update_progress", post(update_task_progress))
        .route("/api/task/delete", post(delete_task))
        .route("/api/reset", post(reset_tasks))
        
        // MCP endpoint
        .route("/mcp", post(mcp_handler))
        
        // CORS
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        
        // State
        .with_state(state)
}

// ============ 主函数 ============

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let config = Config::default();
    let state = Arc::new(AppState::new(config.clone()));
    let router = create_router(state.clone());
    
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    println!("🚀 Vibe MCP Server started on {}", addr);
    println!("📡 Health: http://{}:{}/health", addr, config.port);
    println!("🔐 Auth:   http://{}:{}/api/auth/login", addr, config.port);
    println!("📋 Tasks:  http://{}:{}/api/status", addr, config.port);
    println!("🔌 MCP:    http://{}:{}/mcp", addr, config.port);
    println!("\n📝 Environment variables:");
    println!("   API_KEY={}", config.api_key);
    println!("   JWT_SECRET={}", config.jwt_secret);
    println!("   JWT_EXPIRY_HOURS={}", config.jwt_expiry_hours);
    
    axum::serve(listener, router).await?;
    Ok(())
}
