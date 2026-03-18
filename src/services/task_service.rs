use crate::models::{Task, TaskStageHistory, TaskStatus};
use crate::utils::helpers::{now_millis, validate_status};
use serde::Deserialize;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool, Row};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize, Default)]
pub struct UpdateStateRequest {
    pub task_id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub start_time: Option<i64>,
    #[serde(default)]
    pub estimated_duration: Option<i64>,
    #[serde(default)]
    pub estimated_duration_ms: Option<i64>,
    #[serde(default)]
    pub end_time: Option<i64>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub active_file: Option<String>,
    #[serde(default)]
    pub project_path: Option<String>,
    #[serde(default)]
    pub is_focused: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateProgressRequest {
    pub task_id: String,
    #[serde(default)]
    pub estimated_duration_ms: Option<i64>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub active_file: Option<String>,
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub is_focused: Option<bool>,
}

#[derive(Debug, Clone)]
struct StageEvent {
    stage: String,
    description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskServiceError {
    NotFound,
    InvalidStatus(String),
}

impl fmt::Display for TaskServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Task not found"),
            Self::InvalidStatus(status) => write!(f, "Invalid status: {}", status),
        }
    }
}

#[derive(Clone)]
pub struct TaskService {
    pub tasks: Arc<Mutex<HashMap<String, Task>>>,
    pub stage_histories: Arc<Mutex<HashMap<String, Vec<TaskStageHistory>>>>,
    pub db: Option<Pool<MySql>>,
    pub db_url: String,
}

impl TaskService {
    pub fn new(db_url: String) -> Self {
        let (tasks, stage_histories) = if db_url.is_empty() {
            (HashMap::new(), HashMap::new())
        } else {
            (
                Self::load_tasks_from_db(&db_url),
                Self::load_stage_histories_from_db(&db_url),
            )
        };

        Self {
            tasks: Arc::new(Mutex::new(tasks)),
            stage_histories: Arc::new(Mutex::new(stage_histories)),
            db: None,
            db_url,
        }
    }

    fn normalized_stage(stage: Option<&str>) -> Option<String> {
        stage
            .map(str::trim)
            .filter(|stage| !stage.is_empty())
            .map(ToOwned::to_owned)
    }

    fn normalized_description(description: Option<&str>) -> Option<String> {
        Self::normalized_stage(description)
    }

    fn stage_event_for_state(
        req: &UpdateStateRequest,
        status: Option<TaskStatus>,
        current_stage: Option<&str>,
    ) -> Option<StageEvent> {
        if let Some(stage) = Self::normalized_stage(req.current_stage.as_deref()) {
            return Some(StageEvent {
                stage,
                description: None,
            });
        }

        match status {
            Some(TaskStatus::Completed) => Some(StageEvent {
                stage: Self::normalized_stage(current_stage)
                    .unwrap_or_else(|| "__completed__".to_string()),
                description: None,
            }),
            Some(TaskStatus::Error) => Some(StageEvent {
                stage: Self::normalized_stage(current_stage).unwrap_or_else(|| "error".to_string()),
                description: None,
            }),
            Some(TaskStatus::Cancelled) => Some(StageEvent {
                stage: Self::normalized_stage(current_stage)
                    .unwrap_or_else(|| "cancelled".to_string()),
                description: None,
            }),
            _ => None,
        }
    }

    fn stage_event_for_progress(
        req: &UpdateProgressRequest,
        current_stage: Option<&str>,
        active_file: Option<&str>,
    ) -> Option<StageEvent> {
        let stage = Self::normalized_stage(req.active_file.as_deref())
            .or_else(|| Self::normalized_stage(active_file))?;
        let description = Self::normalized_description(req.current_stage.as_deref())
            .or_else(|| Self::normalized_description(current_stage));

        Some(StageEvent { stage, description })
    }

    fn should_finalize_stage(status: Option<TaskStatus>) -> bool {
        matches!(
            status,
            Some(TaskStatus::Completed | TaskStatus::Error | TaskStatus::Cancelled)
        )
    }

    fn record_stage_history_in_memory(
        &self,
        task_id: &str,
        stage_event: &StageEvent,
        started_at: i64,
        finalize_at: Option<i64>,
    ) {
        let mut histories = self.stage_histories.lock().unwrap();
        let entries = histories.entry(task_id.to_string()).or_default();

        if let Some(last) = entries
            .iter_mut()
            .rev()
            .find(|entry| entry.ended_at.is_none())
        {
            if last.stage == stage_event.stage {
                if stage_event.description.is_some() {
                    last.description = stage_event.description.clone();
                }
                if let Some(ended_at) = finalize_at {
                    last.ended_at = Some(ended_at.max(last.started_at));
                    last.duration = Some((ended_at - last.started_at).max(0));
                }
                return;
            }

            let closed_at = started_at.max(last.started_at);
            last.ended_at = Some(closed_at);
            last.duration = Some((closed_at - last.started_at).max(0));
        }

        let ended_at = finalize_at;
        entries.push(TaskStageHistory {
            task_id: task_id.to_string(),
            stage: stage_event.stage.clone(),
            description: stage_event.description.clone(),
            started_at,
            ended_at,
            duration: ended_at.map(|ended_at| (ended_at - started_at).max(0)),
        });
    }

    pub fn get_task_stage_history(&self, task_id: &str) -> Vec<TaskStageHistory> {
        if let Some(history) = self.stage_histories.lock().unwrap().get(task_id).cloned() {
            return history;
        }

        if self.db_url.is_empty() {
            return Vec::new();
        }

        let history = Self::load_stage_history_from_db(&self.db_url, task_id);
        if !history.is_empty() {
            self.stage_histories
                .lock()
                .unwrap()
                .insert(task_id.to_string(), history.clone());
        }
        history
    }

    async fn persist_task_and_stage_history(
        db_url: String,
        task: Task,
        stage_event: Option<StageEvent>,
        stage_started_at: i64,
        stage_finalize_at: Option<i64>,
    ) {
        let pool = MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await;
        if let Ok(pool) = pool {
            let status_str = format!("{:?}", task.status).to_lowercase();
            if let Ok(mut tx) = pool.begin().await {
                let task_query = sqlx::query(
                    "INSERT INTO vibe_tasks (id, user_id, name, status, source, start_time, end_time, last_heartbeat, estimated_duration_ms, current_stage, ide, window_title, project_path, active_file, is_focused) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE user_id = VALUES(user_id), name = VALUES(name), status = VALUES(status), source = VALUES(source), start_time = VALUES(start_time), end_time = VALUES(end_time), last_heartbeat = VALUES(last_heartbeat), estimated_duration_ms = VALUES(estimated_duration_ms), current_stage = VALUES(current_stage), ide = VALUES(ide), window_title = VALUES(window_title), project_path = VALUES(project_path), active_file = VALUES(active_file), is_focused = VALUES(is_focused)"
                )
                .bind(&task.id)
                .bind(&task.user_id)
                .bind(&task.name)
                .bind(&status_str)
                .bind(&task.source)
                .bind(task.start_time)
                .bind(task.end_time)
                .bind(task.last_heartbeat)
                .bind(task.estimated_duration)
                .bind(&task.current_stage)
                .bind(&task.ide)
                .bind(&task.window_title)
                .bind(&task.project_path)
                .bind(&task.active_file)
                .bind(task.is_focused)
                .execute(&mut *tx)
                .await;

                if task_query.is_err() {
                    let _ = tx.rollback().await;
                    return;
                }

                if let Some(stage_event) = stage_event {
                    let close_previous = sqlx::query(
                        "UPDATE vibe_task_stages SET ended_at = ?, duration = GREATEST(? - started_at, 0) WHERE task_id = ? AND ended_at IS NULL"
                    )
                    .bind(stage_started_at)
                    .bind(stage_started_at)
                    .bind(&task.id)
                    .execute(&mut *tx)
                    .await;

                    if close_previous.is_err() {
                        let _ = tx.rollback().await;
                        return;
                    }

                    let insert_stage = sqlx::query(
                        "INSERT INTO vibe_task_stages (task_id, stage, description, started_at, ended_at, duration) VALUES (?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&task.id)
                    .bind(&stage_event.stage)
                    .bind(&stage_event.description)
                    .bind(stage_started_at)
                    .bind(stage_finalize_at)
                    .bind(stage_finalize_at.map(|ended_at| (ended_at - stage_started_at).max(0)))
                    .execute(&mut *tx)
                    .await;

                    if insert_stage.is_err() {
                        let _ = tx.rollback().await;
                        return;
                    }
                }

                let _ = tx.commit().await;
            }
        }
    }

    fn load_tasks_from_db(db_url: &str) -> HashMap<String, Task> {
        let db_url = db_url.to_string();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::warn!("Failed to create startup DB runtime: {}", error);
                    return HashMap::new();
                }
            };

            runtime.block_on(async move {
                let pool = match MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url)
                    .await
                {
                    Ok(pool) => pool,
                    Err(error) => {
                        tracing::warn!("Failed to connect to database during task load: {}", error);
                        return HashMap::new();
                    }
                };

                let rows = match sqlx::query(
                    "SELECT id, user_id, name, status, source, start_time, end_time, last_heartbeat, estimated_duration_ms, current_stage, ide, window_title, project_path, active_file, is_focused FROM vibe_tasks"
                )
                .fetch_all(&pool)
                .await
                {
                    Ok(rows) => rows,
                    Err(error) => {
                        tracing::warn!("Failed to load tasks from database: {}", error);
                        return HashMap::new();
                    }
                };

                let mut tasks = HashMap::with_capacity(rows.len());
                for row in rows {
                    let status = row
                        .try_get::<String, _>("status")
                        .ok()
                        .and_then(|value| validate_status(&value))
                        .unwrap_or(TaskStatus::Armed);

                    let id: String = match row.try_get("id") {
                        Ok(id) => id,
                        Err(error) => {
                            tracing::warn!("Skipping task row without valid id: {}", error);
                            continue;
                        }
                    };

                    let task = Task {
                        id: id.clone(),
                        user_id: row.try_get("user_id").unwrap_or_default(),
                        name: row.try_get("name").unwrap_or_else(|_| id.clone()),
                        is_focused: row.try_get("is_focused").unwrap_or(false),
                        ide: row.try_get("ide").unwrap_or_else(|_| "IDE".to_string()),
                        window_title: row.try_get("window_title").unwrap_or_else(|_| id.clone()),
                        project_path: row.try_get("project_path").ok(),
                        active_file: row.try_get("active_file").ok(),
                        status,
                        source: row.try_get("source").unwrap_or_else(|_| "mcp".to_string()),
                        start_time: row.try_get::<Option<i64>, _>("start_time").ok().flatten().unwrap_or(0),
                        end_time: row.try_get::<Option<i64>, _>("end_time").ok().flatten(),
                        last_heartbeat: row
                            .try_get::<Option<i64>, _>("last_heartbeat")
                            .ok()
                            .flatten()
                            .unwrap_or(0),
                        estimated_duration: row
                            .try_get::<Option<i64>, _>("estimated_duration_ms")
                            .ok()
                            .flatten(),
                        current_stage: row.try_get("current_stage").ok(),
                    };

                    tasks.insert(id, task);
                }

                tracing::info!("Loaded {} tasks from database", tasks.len());
                tasks
            })
        })
        .join()
        .unwrap_or_else(|_| {
            tracing::warn!("Task loading thread panicked during startup");
            HashMap::new()
        })
    }

    fn load_stage_histories_from_db(db_url: &str) -> HashMap<String, Vec<TaskStageHistory>> {
        let db_url = db_url.to_string();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::warn!("Failed to create startup DB runtime for stage history: {}", error);
                    return HashMap::new();
                }
            };

            runtime.block_on(async move {
                let pool = match MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url)
                    .await
                {
                    Ok(pool) => pool,
                    Err(error) => {
                        tracing::warn!(
                            "Failed to connect to database during stage history load: {}",
                            error
                        );
                        return HashMap::new();
                    }
                };

                let rows = match sqlx::query(
                    "SELECT task_id, stage, description, started_at, ended_at, duration FROM vibe_task_stages ORDER BY task_id ASC, started_at ASC, id ASC"
                )
                .fetch_all(&pool)
                .await
                {
                    Ok(rows) => rows,
                    Err(error) => {
                        tracing::warn!("Failed to load stage histories from database: {}", error);
                        return HashMap::new();
                    }
                };

                let mut histories: HashMap<String, Vec<TaskStageHistory>> = HashMap::new();
                for row in rows {
                    let task_id: String = match row.try_get("task_id") {
                        Ok(task_id) => task_id,
                        Err(error) => {
                            tracing::warn!("Skipping stage row without valid task_id: {}", error);
                            continue;
                        }
                    };

                    histories.entry(task_id.clone()).or_default().push(TaskStageHistory {
                        task_id,
                        stage: row.try_get("stage").unwrap_or_default(),
                        description: row.try_get("description").ok(),
                        started_at: row.try_get("started_at").unwrap_or(0),
                        ended_at: row.try_get("ended_at").ok(),
                        duration: row.try_get("duration").ok(),
                    });
                }

                histories
            })
        })
        .join()
        .unwrap_or_else(|_| {
            tracing::warn!("Stage history loading thread panicked during startup");
            HashMap::new()
        })
    }

    fn load_stage_history_from_db(db_url: &str, task_id: &str) -> Vec<TaskStageHistory> {
        let db_url = db_url.to_string();
        let task_id = task_id.to_string();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::warn!("Failed to create runtime for lazy stage history load: {}", error);
                    return Vec::new();
                }
            };

            runtime.block_on(async move {
                let pool = match MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url)
                    .await
                {
                    Ok(pool) => pool,
                    Err(error) => {
                        tracing::warn!(
                            "Failed to connect to database during lazy stage history load: {}",
                            error
                        );
                        return Vec::new();
                    }
                };

                let rows = match sqlx::query(
                    "SELECT task_id, stage, description, started_at, ended_at, duration FROM vibe_task_stages WHERE task_id = ? ORDER BY started_at ASC, id ASC"
                )
                .bind(&task_id)
                .fetch_all(&pool)
                .await
                {
                    Ok(rows) => rows,
                    Err(error) => {
                        tracing::warn!("Failed to load stage history for task {}: {}", task_id, error);
                        return Vec::new();
                    }
                };

                rows.into_iter()
                    .map(|row| TaskStageHistory {
                        task_id: row.try_get("task_id").unwrap_or_default(),
                        stage: row.try_get("stage").unwrap_or_default(),
                        description: row.try_get("description").ok(),
                        started_at: row.try_get("started_at").unwrap_or(0),
                        ended_at: row.try_get("ended_at").ok(),
                        duration: row.try_get("duration").ok(),
                    })
                    .collect()
            })
        })
        .join()
        .unwrap_or_else(|_| {
            tracing::warn!("Lazy stage history loading thread panicked");
            Vec::new()
        })
    }

    pub async fn init_db(&self) -> Option<Pool<MySql>> {
        if self.db_url.is_empty() {
            return None;
        }
        match MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&self.db_url)
            .await
        {
            Ok(pool) => {
                tracing::info!("Connected to database");
                Some(pool)
            }
            Err(e) => {
                tracing::warn!("Failed to connect to database: {}", e);
                None
            }
        }
    }

    pub fn get_tasks(&self, user_id: Option<&str>) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        let mut tasks_vec: Vec<Task> = if let Some(uid) = user_id {
            tasks
                .values()
                .filter(|t| &t.user_id == uid)
                .cloned()
                .collect()
        } else {
            tasks.values().cloned().collect()
        };
        tasks_vec.sort_by(|a, b| b.id.cmp(&a.id));
        tasks_vec
    }

    pub fn get_task(&self, task_id: &str, user_id: &str) -> Option<Task> {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .get(task_id)
            .filter(|task| task.user_id == user_id)
            .cloned()
    }

    pub fn update_task_status(
        &self,
        req: &UpdateStateRequest,
        user_id: &str,
    ) -> Result<(), TaskServiceError> {
        let mut tasks = self.tasks.lock().unwrap();

        let task = tasks.entry(req.task_id.clone()).or_insert_with(|| {
            Task::new(
                req.task_id.clone(),
                user_id.to_string(),
                req.task_id.clone(),
                "IDE".to_string(),
                req.task_id.clone(),
            )
        });

        if &task.user_id != user_id {
            return Err(TaskServiceError::NotFound);
        }

        let now = now_millis();
        let mut history_status = None;
        if let Some(status_str) = &req.status {
            let new_status = validate_status(status_str)
                .ok_or_else(|| TaskServiceError::InvalidStatus(status_str.clone()))?;
            history_status = Some(new_status);
            task.status = new_status;

            match new_status {
                TaskStatus::Running => {
                    if let Some(start_time) = req.start_time {
                        task.start_time = start_time;
                    } else if task.start_time == 0 {
                        task.start_time = now;
                    }
                }
                TaskStatus::Completed | TaskStatus::Error | TaskStatus::Cancelled => {
                    task.end_time = Some(req.end_time.unwrap_or(now));
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

        if let Some(start_time) = req.start_time {
            task.start_time = start_time;
        }
        if let Some(est) = req.estimated_duration {
            task.estimated_duration = Some(est);
        }
        if let Some(est) = req.estimated_duration_ms {
            task.estimated_duration = Some(est);
        }
        if let Some(end_time) = req.end_time {
            task.end_time = Some(end_time);
        }
        if let Some(stage) = &req.current_stage {
            task.current_stage = Some(stage.clone());
        }
        if let Some(source) = &req.source {
            task.source = source.clone();
        }
        if let Some(window_title) = &req.window_title {
            task.window_title = window_title.clone();
        }
        if let Some(active_file) = &req.active_file {
            task.active_file = Some(active_file.clone());
        }
        if let Some(project_path) = &req.project_path {
            task.project_path = Some(project_path.clone());
        }
        if let Some(is_focused) = req.is_focused {
            task.is_focused = is_focused;
        }

        task.last_heartbeat = now;

        let stage_event =
            Self::stage_event_for_state(req, history_status, task.current_stage.as_deref());
        let stage_started_at = if Self::should_finalize_stage(history_status) {
            task.end_time.unwrap_or(now)
        } else if history_status == Some(TaskStatus::Running) && task.start_time > 0 {
            task.start_time
        } else {
            now
        };
        let stage_finalize_at = if Self::should_finalize_stage(history_status) {
            Some(stage_started_at)
        } else {
            None
        };

        let task_clone = task.clone();
        let db_url = self.db_url.clone();
        drop(tasks);

        if let Some(stage) = &stage_event {
            self.record_stage_history_in_memory(
                &task_clone.id,
                stage,
                stage_started_at,
                stage_finalize_at,
            );
        }

        if !db_url.is_empty() {
            let stage_event_clone = stage_event.clone();
            tokio::spawn(async move {
                Self::persist_task_and_stage_history(
                    db_url,
                    task_clone,
                    stage_event_clone,
                    stage_started_at,
                    stage_finalize_at,
                )
                .await;
            });
        }

        Ok(())
    }

    pub fn update_task_progress(
        &self,
        req: &UpdateProgressRequest,
        user_id: &str,
    ) -> Result<(), TaskServiceError> {
        let mut tasks = self.tasks.lock().unwrap();

        let task = tasks
            .get_mut(&req.task_id)
            .ok_or(TaskServiceError::NotFound)?;

        if &task.user_id != user_id {
            return Err(TaskServiceError::NotFound);
        }

        let now = now_millis();
        if let Some(est) = req.estimated_duration_ms {
            task.estimated_duration = Some(est);
        }
        if let Some(stage) = &req.current_stage {
            task.current_stage = Some(stage.clone());
        }
        if let Some(active_file) = &req.active_file {
            task.active_file = Some(active_file.clone());
        }
        if let Some(window_title) = &req.window_title {
            task.window_title = window_title.clone();
        }
        if let Some(is_focused) = req.is_focused {
            task.is_focused = is_focused;
        }

        task.last_heartbeat = now;

        let stage_event = Self::stage_event_for_progress(
            req,
            task.current_stage.as_deref(),
            task.active_file.as_deref(),
        );
        let task_clone = task.clone();
        let db_url = self.db_url.clone();
        drop(tasks);

        if let Some(stage) = &stage_event {
            self.record_stage_history_in_memory(&task_clone.id, stage, now, None);
        }

        if !db_url.is_empty() {
            let stage_event_clone = stage_event.clone();
            tokio::spawn(async move {
                Self::persist_task_and_stage_history(
                    db_url,
                    task_clone,
                    stage_event_clone,
                    now,
                    None,
                )
                .await;
            });
        }

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
}

#[cfg(test)]
mod tests {
    use super::{TaskService, UpdateProgressRequest, UpdateStateRequest};
    use crate::models::TaskStatus;

    fn create_task_service() -> TaskService {
        TaskService::new(String::new())
    }

    #[test]
    fn records_stage_history_for_current_stage_updates() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-1".to_string(),
                    status: Some("running".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-1".to_string(),
                    current_stage: Some("Planning".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-1".to_string(),
                    current_stage: Some("Implementing".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let history = service.get_task_stage_history("task-1");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].stage, "Planning");
        assert_eq!(history[0].description, Some("Planning".to_string()));
        assert_eq!(history[1].stage, "Implementing");
        assert_eq!(history[1].description, Some("Implementing".to_string()));
        assert!(history[0].ended_at.is_some());
        assert_eq!(history[1].ended_at, None);
    }

    #[test]
    fn prefers_explicit_current_stage_over_active_file_in_progress_history() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-priority".to_string(),
                    status: Some("running".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-priority".to_string(),
                    current_stage: Some("refactoring code".to_string()),
                    active_file: Some("rpc_query_nds.cc".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let history = service.get_task_stage_history("task-priority");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].stage, "rpc_query_nds.cc");
        assert_eq!(history[0].description, Some("refactoring code".to_string()));
    }

    #[test]
    fn falls_back_to_active_file_for_progress_history_when_stage_missing() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-file-fallback".to_string(),
                    status: Some("running".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-file-fallback".to_string(),
                    active_file: Some("src/lib.rs".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let history = service.get_task_stage_history("task-file-fallback");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].stage, "src/lib.rs");
        assert_eq!(history[0].description, None);
    }

    #[test]
    fn does_not_use_current_stage_as_progress_stage_without_active_file() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-no-file".to_string(),
                    status: Some("running".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-no-file".to_string(),
                    current_stage: Some("refactoring code".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let history = service.get_task_stage_history("task-no-file");
        assert!(history.is_empty());
    }

    #[test]
    fn updates_description_for_repeated_progress_on_same_stage() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-desc".to_string(),
                    status: Some("running".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-desc".to_string(),
                    active_file: Some("src/lib.rs".to_string()),
                    current_stage: Some("Drafting changes".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-desc".to_string(),
                    active_file: Some("src/lib.rs".to_string()),
                    current_stage: Some("Polishing edge cases".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let history = service.get_task_stage_history("task-desc");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].stage, "src/lib.rs");
        assert_eq!(
            history[0].description,
            Some("Polishing edge cases".to_string())
        );
    }

    #[test]
    fn skips_default_running_stage_history_noise() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-noise".to_string(),
                    status: Some("running".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let history = service.get_task_stage_history("task-noise");
        assert!(history.is_empty());
    }

    #[test]
    fn updates_desktop_metadata_from_state_and_progress_requests() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-meta".to_string(),
                    status: Some("running".to_string()),
                    project_path: Some("/tmp/project".to_string()),
                    active_file: Some("src/main.rs".to_string()),
                    window_title: Some("main.rs - Cursor".to_string()),
                    is_focused: Some(true),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_progress(
                &UpdateProgressRequest {
                    task_id: "task-meta".to_string(),
                    active_file: Some("src/lib.rs".to_string()),
                    window_title: Some("lib.rs - Cursor".to_string()),
                    is_focused: Some(false),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let task = service.get_task("task-meta", "user-1").unwrap();
        assert_eq!(task.project_path.as_deref(), Some("/tmp/project"));
        assert_eq!(task.active_file.as_deref(), Some("src/lib.rs"));
        assert_eq!(task.window_title, "lib.rs - Cursor");
        assert!(!task.is_focused);
    }

    #[test]
    fn records_terminal_status_as_closed_stage_history() {
        let service = create_task_service();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-2".to_string(),
                    status: Some("running".to_string()),
                    current_stage: Some("Coding".to_string()),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        service
            .update_task_status(
                &UpdateStateRequest {
                    task_id: "task-2".to_string(),
                    status: Some("completed".to_string()),
                    end_time: Some(1_710_000_000_000),
                    ..Default::default()
                },
                "user-1",
            )
            .unwrap();

        let tasks = service.get_tasks(Some("user-1"));
        assert_eq!(tasks[0].status, TaskStatus::Completed);

        let history = service.get_task_stage_history("task-2");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].stage, "Coding");
        assert_eq!(history[0].description, None);
        assert_eq!(history[1].stage, "__completed__");
        assert_eq!(history[1].description, None);
        assert_eq!(history[1].ended_at, Some(1_710_000_000_000));
        assert_eq!(history[1].duration, Some(0));
    }
}
