use crate::models::{Task, TaskStatus};
use crate::utils::helpers::{now_millis, validate_status};
use serde::Deserialize;
use sqlx::{mysql::MySqlPoolOptions, Pool, MySql, Row};
use std::collections::HashMap;
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

#[derive(Clone)]
pub struct TaskService {
    pub tasks: Arc<Mutex<HashMap<String, Task>>>,
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
            db: None,
            db_url,
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
            tasks.values().filter(|t| &t.user_id == uid).cloned().collect()
        } else {
            tasks.values().cloned().collect()
        };
        tasks_vec.sort_by(|a, b| b.id.cmp(&a.id));
        tasks_vec
    }

    pub fn update_task_status(&self, req: &UpdateStateRequest, user_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
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
        
        if &task.user_id != user_id {
            return Err("Task not found".to_string());
        }
        
        if let Some(status_str) = &req.status {
            let new_status = validate_status(status_str)
                .ok_or_else(|| format!("Invalid status: {}", status_str))?;
            
            task.status = new_status;
            
            match new_status {
                TaskStatus::Running => {
                    if let Some(start_time) = req.start_time {
                        task.start_time = start_time;
                    } else if task.start_time == 0 {
                        task.start_time = now_millis();
                    }
                }
                TaskStatus::Completed | TaskStatus::Error | TaskStatus::Cancelled => {
                    task.end_time = Some(req.end_time.unwrap_or_else(now_millis));
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
        
        task.last_heartbeat = now_millis();
        
        // Save to database asynchronously
        let task_clone = task.clone();
        let db_url = self.db_url.clone();
        drop(tasks);
        
        if !db_url.is_empty() {
            tokio::spawn(async move {
                let pool = MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url).await;
                if let Ok(pool) = pool {
                    let status_str = format!("{:?}", task_clone.status).to_lowercase();
                    let _ = sqlx::query(
                        "INSERT INTO vibe_tasks (id, user_id, name, status, source, start_time, end_time, last_heartbeat, estimated_duration_ms, current_stage) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE status = ?, end_time = ?, last_heartbeat = ?, estimated_duration_ms = ?, current_stage = ?"
                    )
                    .bind(&task_clone.id)
                    .bind(&task_clone.user_id)
                    .bind(&task_clone.name)
                    .bind(&status_str)
                    .bind(&task_clone.source)
                    .bind(task_clone.start_time)
                    .bind(task_clone.end_time)
                    .bind(task_clone.last_heartbeat)
                    .bind(task_clone.estimated_duration)
                    .bind(&task_clone.current_stage)
                    .bind(&status_str)
                    .bind(task_clone.end_time)
                    .bind(task_clone.last_heartbeat)
                    .bind(task_clone.estimated_duration)
                    .bind(&task_clone.current_stage)
                    .execute(&pool).await;
                }
            });
        }
        
        Ok(())
    }

    pub fn update_task_progress(&self, req: &UpdateProgressRequest, user_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
        let task = tasks.get_mut(&req.task_id)
            .ok_or_else(|| format!("Task not found: {}", req.task_id))?;
        
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
        
        let task_clone = task.clone();
        let db_url = self.db_url.clone();
        drop(tasks);
        
        if !db_url.is_empty() {
            tokio::spawn(async move {
                let pool = MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url).await;
                if let Ok(pool) = pool {
                    let status_str = format!("{:?}", task_clone.status).to_lowercase();
                    let _ = sqlx::query(
                        "INSERT INTO vibe_tasks (id, user_id, name, status, source, start_time, end_time, last_heartbeat, estimated_duration_ms, current_stage) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE status = ?, end_time = ?, last_heartbeat = ?, estimated_duration_ms = ?, current_stage = ?"
                    )
                    .bind(&task_clone.id)
                    .bind(&task_clone.user_id)
                    .bind(&task_clone.name)
                    .bind(&status_str)
                    .bind(&task_clone.source)
                    .bind(task_clone.start_time)
                    .bind(task_clone.end_time)
                    .bind(task_clone.last_heartbeat)
                    .bind(task_clone.estimated_duration)
                    .bind(&task_clone.current_stage)
                    .bind(&status_str)
                    .bind(task_clone.end_time)
                    .bind(task_clone.last_heartbeat)
                    .bind(task_clone.estimated_duration)
                    .bind(&task_clone.current_stage)
                    .execute(&pool).await;
                }
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
