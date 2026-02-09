use crate::models::TaskStatus;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::utils::helpers::{now_millis, validate_status};

#[derive(Clone)]
pub struct TaskService {
    pub tasks: Arc<Mutex<HashMap<String, crate::models::Task>>>,
}

impl TaskService {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_tasks(&self, user_id: Option<&str>) -> Vec<crate::models::Task> {
        let tasks = self.tasks.lock().unwrap();
        let mut tasks_vec: Vec<crate::models::Task> = if let Some(uid) = user_id {
            tasks.values().filter(|t| &t.user_id == uid).cloned().collect()
        } else {
            tasks.values().cloned().collect()
        };
        tasks_vec.sort_by(|a, b| b.id.cmp(&a.id));
        tasks_vec
    }

    pub fn update_task_status(&self, task_id: &str, status: Option<&str>, source: Option<&str>, estimated_duration: Option<i64>, current_stage: Option<&str>, user_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
        let task = tasks
            .entry(task_id.to_string())
            .or_insert_with(|| {
                crate::models::Task::new(
                    task_id.to_string(),
                    user_id.to_string(),
                    task_id.to_string(),
                    "IDE".to_string(),
                    task_id.to_string(),
                )
            });
        
        if &task.user_id != user_id {
            return Err("Task not found".to_string());
        }
        
        if let Some(status_str) = status {
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
        
        if let Some(est) = estimated_duration {
            task.estimated_duration = Some(est);
        }
        if let Some(stage) = current_stage {
            task.current_stage = Some(stage.to_string());
        }
        
        task.last_heartbeat = now_millis();
        Ok(())
    }

    pub fn update_task_progress(&self, task_id: &str, estimated_duration_ms: Option<i64>, current_stage: Option<&str>, user_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        
        let task = tasks.get_mut(task_id)
            .ok_or_else(|| format!("Task not found: {}", task_id))?;
        
        if &task.user_id != user_id {
            return Err("Task not found".to_string());
        }
        
        if let Some(est) = estimated_duration_ms {
            task.estimated_duration = Some(est);
        }
        if let Some(stage) = current_stage {
            task.current_stage = Some(stage.to_string());
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

    pub fn calculate_progress(&self, task: &crate::models::Task) -> u32 {
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
