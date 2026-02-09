//! Vibe MCP Server - AI Task Status Tracker
//! 基于 MCP 协议的 AI 任务状态跟踪服务

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::cors::{Any, CorsLayer};

// ============ 配置 ============

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3010,
            api_key: std::env::var("API_KEY").unwrap_or_else(|_| "vibe-mcp-secret-key".to_string()),
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
    pub name: String,
    pub is_focused: bool,
    pub ide: String,
    pub window_title: String,
    pub project_path: Option<String>,
    pub active_file: Option<String>,
    pub status: TaskStatus,
    pub source: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub last_heartbeat: u64,
    pub estimated_duration: Option<u64>,
    pub current_stage: Option<String>,
}

impl Task {
    pub fn new(id: String, name: String, ide: String, window_title: String) -> Self {
        let now = now_millis();
        Self {
            id,
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

// ============ 请求结构 ============

#[derive(Debug, Deserialize, Default)]
pub struct UpdateStateRequest {
    pub task_id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub estimated_duration: Option<u64>,
    #[serde(default)]
    pub current_stage: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateProgressRequest {
    pub task_id: String,
    #[serde(default)]
    pub estimated_duration_ms: Option<u64>,
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

// ============ 服务状态 ============

#[derive(Clone)]
pub struct AppState {
    pub tasks: Arc<Mutex<HashMap<String, Task>>>,
    pub config: Config,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
}

// ============ 工具函数 ============

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
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

// ============ 业务逻辑 ============

impl AppState {
    pub fn get_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        let mut tasks_vec: Vec<Task> = tasks.values().cloned().collect();
        tasks_vec.sort_by(|a, b| b.id.cmp(&a.id));
        tasks_vec
    }

    pub fn update_task_status(&self, req: &UpdateStateRequest) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
        let task = tasks.get_mut(&req.task_id)
            .ok_or_else(|| format!("Task not found: {}", req.task_id))?;
        
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

    pub fn update_task_progress(&self, req: &UpdateProgressRequest) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
        let task = tasks.get_mut(&req.task_id)
            .ok_or_else(|| format!("Task not found: {}", req.task_id))?;
        
        if let Some(est) = req.estimated_duration_ms {
            task.estimated_duration = Some(est);
        }
        if let Some(stage) = &req.current_stage {
            task.current_stage = Some(stage.clone());
        }
        
        task.last_heartbeat = now_millis();
        Ok(())
    }

    pub fn reset_tasks(&self, task_id: Option<String>) {
        let mut tasks = self.tasks.lock().unwrap();
        match task_id {
            Some(id) => { tasks.remove(&id); }
            None => { tasks.clear(); }
        }
    }

    pub fn delete_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.remove(task_id).is_some()
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
}

// ============ API Key 鉴权 ============

async fn authenticate(
    headers: &HeaderMap,
    api_key: &str,
) -> Result<(), StatusCode> {
    if let Some(key) = headers.get("x-api-key") {
        if key.to_str().map_or(false, |s| s == api_key) {
            return Ok(());
        }
    }
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") && &auth_str[7..] == api_key {
                return Ok(());
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

// ============ API 处理器 ============

async fn get_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let tasks = state.get_tasks();
    Json(json!({ "tasks": tasks, "taskCount": tasks.len() }))
}

async fn update_task_state(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Extension(api_key): axum::extract::Extension<String>,
    Json(req): Json<UpdateStateRequest>,
) -> Result<Json<Value>, StatusCode> {
    authenticate(&headers, &api_key).await?;
    state.update_task_status(&req).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn update_task_progress(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Extension(api_key): axum::extract::Extension<String>,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<Json<Value>, StatusCode> {
    authenticate(&headers, &api_key).await?;
    state.update_task_progress(&req).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn delete_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Extension(api_key): axum::extract::Extension<String>,
    Json(req): Json<DeleteTaskRequest>,
) -> Result<Json<Value>, StatusCode> {
    authenticate(&headers, &api_key).await?;
    if state.delete_task(&req.task_id) {
        Ok(Json(json!({"status": "ok"})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn reset_tasks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Extension(api_key): axum::extract::Extension<String>,
    Json(req): Json<ResetRequest>,
) -> Result<Json<Value>, StatusCode> {
    authenticate(&headers, &api_key).await?;
    state.reset_tasks(req.task_id);
    Ok(Json(json!({"status": "ok"})))
}

// ============ MCP 协议处理器 ============

async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Extension(api_key): axum::extract::Extension<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, StatusCode> {
    authenticate(&headers, &api_key).await?;
    
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
                { "name": "list_tasks", "description": "Get all tasks", 
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
                    let tasks = state.get_tasks();
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
                    match state.update_task_status(&req) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Task {} -> {}", task_id, status) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "update_task_progress" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let req = UpdateProgressRequest {
                        task_id: task_id.to_string(),
                        estimated_duration_ms: args.get("estimated_duration_ms").and_then(|v| v.as_u64()),
                        current_stage: args.get("current_stage").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    };
                    match state.update_task_progress(&req) {
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
    let api_key = state.config.api_key.clone();
    
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/status", get(get_status))
        .route("/api/task/update_state", post(update_task_state))
        .route("/api/task/update_progress", post(update_task_progress))
        .route("/api/task/delete", post(delete_task))
        .route("/api/reset", post(reset_tasks))
        .route("/mcp", post(mcp_handler))
        .layer(axum::extract::Extension(api_key))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
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
    println!("📡 API: http://{}:{}/api/status", addr, config.port);
    println!("🔌 MCP: http://{}:{}/mcp", addr, config.port);
    println!("🔑 Key: {}", config.api_key);
    
    axum::serve(listener, router).await?;
    Ok(())
}
