use serde::{Deserialize, Serialize};

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
        let now = chrono::Utc::now().timestamp_millis();
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
