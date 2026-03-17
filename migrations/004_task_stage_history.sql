-- Migration: Create vibe_task_stages table

CREATE TABLE IF NOT EXISTS vibe_task_stages (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    task_id VARCHAR(36) NOT NULL,
    stage TEXT NOT NULL,
    started_at BIGINT NOT NULL,
    ended_at BIGINT NULL,
    duration BIGINT NULL,
    created_at BIGINT NOT NULL DEFAULT (UNIX_TIMESTAMP() * 1000),
    updated_at BIGINT NOT NULL DEFAULT (UNIX_TIMESTAMP() * 1000),
    FOREIGN KEY (task_id) REFERENCES vibe_tasks(id) ON DELETE CASCADE,
    INDEX idx_task_stage_task_id (task_id),
    INDEX idx_task_stage_started_at (started_at),
    INDEX idx_task_stage_ended_at (ended_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
