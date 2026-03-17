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
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateProgressRequest {
    pub task_id: String,
    #[serde(default)]
    pub estimated_duration_ms: Option<i64>,
    #[serde(default)]
    pub current_stage: Option<String>,
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
        let tasks = if db_url.is_empty() {
            HashMap::new()
        } else {
            Self::load_tasks_from_db(&db_url)
        };

        Self {
            tasks: Arc::new(Mutex::new(tasks)),
            stage_histories: Arc::new(Mutex::new(HashMap::new())),
            db: None,
            db_url,
        }
    }

    fn status_to_history_stage(status: TaskStatus, current_stage: Option<&str>) -> String {
        match status {
            TaskStatus::Armed => "armed".to_string(),
            TaskStatus::Running => current_stage.unwrap_or("running").to_string(),
            TaskStatus::Completed => current_stage.unwrap_or("__completed__").to_string(),
            TaskStatus::Error => current_stage.unwrap_or("error").to_string(),
            TaskStatus::Cancelled => current_stage.unwrap_or("cancelled").to_string(),
        }
    }

    fn should_record_stage_history(req: &UpdateStateRequest) -> bool {
        req.current_stage.is_some() || req.status.is_some()
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
        stage: &str,
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
            last.ended_at = Some(started_at);
            last.duration = Some((started_at - last.started_at).max(0));
        }

        let ended_at = finalize_at;
        entries.push(TaskStageHistory {
            task_id: task_id.to_string(),
            stage: stage.to_string(),
            started_at,
            ended_at,
            duration: ended_at.map(|ended_at| (ended_at - started_at).max(0)),
        });
    }

    pub fn get_task_stage_history(&self, task_id: &str) -> Vec<TaskStageHistory> {
        self.stage_histories
            .lock()
            .unwrap()
            .get(task_id)
            .cloned()
            .unwrap_or_default()
    }

    async fn persist_task_and_stage_history(
        db_url: String,
        task: Task,
        stage_event: Option<String>,
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
                    "INSERT INTO vibe_tasks (id, user_id, name, status, source, start_time, end_time, last_heartbeat, estimated_duration_ms, current_stage) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE status = ?, end_time = ?, last_heartbeat = ?, estimated_duration_ms = ?, current_stage = ?"
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
                .bind(&status_str)
                .bind(task.end_time)
                .bind(task.last_heartbeat)
                .bind(task.estimated_duration)
                .bind(&task.current_stage)
                .execute(&mut *tx)
                .await;

                if task_query.is_err() {
                    let _ = tx.rollback().await;
                    return;
                }

                if let Some(stage) = stage_event {
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
                        "INSERT INTO vibe_task_stages (task_id, stage, started_at, ended_at, duration) VALUES (?, ?, ?, ?, ?)"
                    )
                    .bind(&task.id)
                    .bind(&stage)
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

        task.last_heartbeat = now;

        let stage_event = if Self::should_record_stage_history(req) {
            Some(req.current_stage.clone().unwrap_or_else(|| {
                Self::status_to_history_stage(task.status, task.current_stage.as_deref())
            }))
        } else {
            None
        };
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

        task.last_heartbeat = now;

        let stage_event = req.current_stage.clone();
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
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].stage, "running");
        assert_eq!(history[1].stage, "Planning");
        assert_eq!(history[2].stage, "Implementing");
        assert!(history[0].ended_at.is_some());
        assert!(history[1].ended_at.is_some());
        assert_eq!(history[2].ended_at, None);
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
        assert_eq!(history[1].stage, "__completed__");
        assert_eq!(history[1].ended_at, Some(1_710_000_000_000));
        assert_eq!(history[1].duration, Some(0));
    }
}
