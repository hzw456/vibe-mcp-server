use crate::services::task_service::{UpdateProgressRequest, UpdateStateRequest};
use crate::models::TaskStatus;
use crate::utils::helpers::{validate_status, now_millis};
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct DeleteTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ResetRequest {
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncTaskRequest {
    pub task_id: String,
    pub status: String,
    #[serde(default)]
    pub start_time: Option<i64>,
    #[serde(default)]
    pub estimated_duration_ms: Option<i64>,
    #[serde(default)]
    pub end_time: Option<i64>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub user_email: Option<String>,
}

pub async fn get_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let user_id = crate::utils::helpers::authenticate_jwt(&headers, &state).await?;
    let tasks = state.task_service.get_tasks(Some(&user_id));
    Ok(Json(json!({ "tasks": tasks, "taskCount": tasks.len() })))
}

pub async fn get_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let user_id = crate::utils::helpers::authenticate_jwt(&headers, &state).await?;
    let tasks: Vec<_> = state
        .task_service
        .get_tasks(Some(&user_id))
        .into_iter()
        .filter(|task| task.status == TaskStatus::Completed)
        .collect();
    Ok(Json(json!({ "tasks": tasks, "taskCount": tasks.len() })))
}

pub async fn update_task_state(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateStateRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = crate::utils::helpers::authenticate_jwt(&headers, &state).await?;
    state.task_service.update_task_status(&req, &user_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({"status": "ok"})))
}

pub async fn update_task_progress(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = crate::utils::helpers::authenticate_jwt(&headers, &state).await?;
    state.task_service.update_task_progress(&req, &user_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({"status": "ok"})))
}

pub async fn sync_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<SyncTaskRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Authenticate via API key
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    let user_id = match api_key {
        Some(key) => {
            state.user_service.find_user_by_api_key(&key)
                .map(|user| user.id)
        },
        None => {
            // Fallback to user_email for backward compatibility (deprecated)
            let user_email = req
                .user_email
                .as_deref()
                .filter(|email| !email.trim().is_empty())
                .unwrap_or("test@vibe.app");
            state.user_service.find_user_by_email(user_email).map(|user| user.id)
        }
    }.ok_or(StatusCode::UNAUTHORIZED)?;
    
    let status = validate_status(&req.status).ok_or(StatusCode::BAD_REQUEST)?;
    
    let update_req = UpdateStateRequest {
        task_id: req.task_id,
        status: Some(format!("{:?}", status).to_lowercase()),
        source: Some("sync".to_string()),
        start_time: req.start_time,
        estimated_duration: None,
        estimated_duration_ms: req.estimated_duration_ms,
        end_time: req.end_time,
        current_stage: req.current_stage,
    };

    state
        .task_service
        .update_task_status(&update_req, &user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({"status": "ok"})))
}

pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<DeleteTaskRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = crate::utils::helpers::authenticate_jwt(&headers, &state).await?;
    if state.task_service.delete_task(&req.task_id, &user_id) {
        Ok(Json(json!({"status": "ok"})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn reset_tasks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ResetRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = crate::utils::helpers::authenticate_jwt(&headers, &state).await?;
    state.task_service.reset_tasks(req.task_id, &user_id);
    Ok(Json(json!({"status": "ok"})))
}

/// MCP Protocol Handler (JSON-RPC 2.0)
/// Supports both JWT and API Key authentication
pub async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, StatusCode> {
    // Authenticate: try API key first, then JWT
    let user_id = {
        if let Some(api_key) = headers.get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
        {
            state.user_service.find_user_by_api_key(&api_key)
                .map(|user| user.id)
        } else {
            // Try JWT
            crate::utils::helpers::authenticate_jwt(&headers, &state).await.ok()
        }
    }.ok_or(StatusCode::UNAUTHORIZED)?;
    
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
                { "name": "task_start", "description": "Start a new task",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" }, "name": { "type": "string" }, "description": { "type": "string" } }, "required": ["task_id", "name"] } },
                { "name": "task_progress", "description": "Update task progress",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" }, "progress": { "type": "number" }, "current_stage": { "type": "string" } }, "required": ["task_id"] } },
                { "name": "task_complete", "description": "Mark task as completed",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" }, "current_stage": { "type": "string" } }, "required": ["task_id"] } },
                { "name": "task_error", "description": "Mark task as error",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" }, "message": { "type": "string" } }, "required": ["task_id"] } },
                { "name": "task_cancel", "description": "Cancel a task",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" } }, "required": ["task_id"] } },
                { "name": "task_update", "description": "Update task status",
                  "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" }, "status": { "type": "string" }, "current_stage": { "type": "string" } }, "required": ["task_id", "status"] } }
            ]
        }),
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_default();
            
            match tool_name {
                "list_tasks" => {
                    let tasks = state.task_service.get_tasks(Some(&user_id));
                    let list: Vec<Value> = tasks.iter().map(|t| json!({
                        "id": t.id, "name": t.name, "ide": t.ide, "status": format!("{:?}", t.status).to_lowercase(),
                        "progress": state.task_service.calculate_progress(t), "source": t.source, "current_stage": t.current_stage
                    })).collect();
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&list).unwrap() }] })
                }
                "task_start" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("Task");
                    let description = args.get("description").and_then(|v| v.as_str()).unwrap_or(name);
                    
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(),
                        status: Some("running".to_string()),
                        source: Some("mcp".to_string()),
                        start_time: Some(now_millis()),
                        current_stage: Some(description.to_string()),
                        ..Default::default()
                    };
                    match state.task_service.update_task_status(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Task {} started", task_id) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "task_progress" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let current_stage = args.get("current_stage").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let progress = args.get("progress").and_then(|v| v.as_f64());
                    
                    let stage = current_stage.unwrap_or_else(|| format!("Progress: {}%", progress.unwrap_or(0.0) as i32));
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(),
                        status: Some("running".to_string()),
                        source: Some("mcp".to_string()),
                        current_stage: Some(stage),
                        ..Default::default()
                    };
                    match state.task_service.update_task_status(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Progress updated for {}", task_id) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "task_complete" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let current_stage = args.get("current_stage").and_then(|v| v.as_str()).unwrap_or("Task completed");
                    
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(),
                        status: Some("completed".to_string()),
                        source: Some("mcp".to_string()),
                        current_stage: Some(current_stage.to_string()),
                        end_time: Some(now_millis()),
                        ..Default::default()
                    };
                    match state.task_service.update_task_status(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Task {} completed", task_id) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "task_error" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("Task failed");
                    
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(),
                        status: Some("error".to_string()),
                        source: Some("mcp".to_string()),
                        current_stage: Some(message.to_string()),
                        end_time: Some(now_millis()),
                        ..Default::default()
                    };
                    match state.task_service.update_task_status(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Task {} marked as error", task_id) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "task_cancel" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(),
                        status: Some("cancelled".to_string()),
                        source: Some("mcp".to_string()),
                        current_stage: Some("Task cancelled".to_string()),
                        ..Default::default()
                    };
                    match state.task_service.update_task_status(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Task {} cancelled", task_id) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "task_update" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let status = args.get("status").and_then(|v| v.as_str()).unwrap_or("running");
                    let current_stage = args.get("current_stage").and_then(|v| v.as_str()).map(|s| s.to_string());
                    
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(),
                        status: Some(status.to_string()),
                        source: Some("mcp".to_string()),
                        current_stage,
                        ..Default::default()
                    };
                    match state.task_service.update_task_status(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Task {} updated to {}", task_id, status) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                "update_task_status" => {
                    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    let status = args.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let req = UpdateStateRequest {
                        task_id: task_id.to_string(), status: Some(status.to_string()), 
                        source: Some("mcp".to_string()), ..Default::default()
                    };
                    match state.task_service.update_task_status(&req, &user_id) {
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
                    match state.task_service.update_task_progress(&req, &user_id) {
                        Ok(_) => json!({ "content": [{ "type": "text", "text": format!("Progress updated for {}", task_id) }] }),
                        Err(e) => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32602, "message": e }, "id": req_id })))
                    }
                }
                _ => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32601, "message": format!("Unknown tool: {}", tool_name) }, "id": req_id })))
            }
        }
        _ => return Ok(Json(json!({ "jsonrpc": "2.0", "error": { "code": -32601, "message": format!("Method: {}", method) }, "id": req_id })))
    };
    
    Ok(Json(json!({ "jsonrpc": "2.0", "result": result, "id": req_id })))
}

pub async fn health() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::SyncTaskRequest;
    use serde_json::json;

    #[test]
    fn sync_task_request_accepts_expected_payload() {
        let req: SyncTaskRequest = serde_json::from_value(json!({
            "task_id": "task-123",
            "status": "running",
            "start_time": 1710000000000_i64,
            "estimated_duration_ms": 300000_i64,
            "end_time": 1710000300000_i64,
            "current_stage": "Analyzing",
            "user_email": "user@example.com"
        }))
        .unwrap();

        assert_eq!(req.task_id, "task-123");
        assert_eq!(req.status, "running");
        assert_eq!(req.start_time, Some(1710000000000));
        assert_eq!(req.estimated_duration_ms, Some(300000));
        assert_eq!(req.end_time, Some(1710000300000));
        assert_eq!(req.current_stage.as_deref(), Some("Analyzing"));
        assert_eq!(req.user_email.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn sync_task_request_allows_optional_fields_to_be_omitted() {
        let req: SyncTaskRequest = serde_json::from_value(json!({
            "task_id": "task-123",
            "status": "completed"
        }))
        .unwrap();

        assert_eq!(req.task_id, "task-123");
        assert_eq!(req.status, "completed");
        assert_eq!(req.start_time, None);
        assert_eq!(req.estimated_duration_ms, None);
        assert_eq!(req.end_time, None);
        assert_eq!(req.current_stage, None);
        assert_eq!(req.user_email, None);
    }

    #[test]
    fn sync_task_request_requires_task_id_and_status() {
        assert!(serde_json::from_value::<SyncTaskRequest>(json!({
            "status": "running"
        }))
        .is_err());

        assert!(serde_json::from_value::<SyncTaskRequest>(json!({
            "task_id": "task-123"
        }))
        .is_err());
    }
}
