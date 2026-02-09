pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn validate_status(status: &str) -> Option<crate::models::TaskStatus> {
    match status.to_lowercase().as_str() {
        "armed" => Some(crate::models::TaskStatus::Armed),
        "running" => Some(crate::models::TaskStatus::Running),
        "completed" => Some(crate::models::TaskStatus::Completed),
        "error" => Some(crate::models::TaskStatus::Error),
        "cancelled" => Some(crate::models::TaskStatus::Cancelled),
        _ => None,
    }
}

pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() >= 5
}
