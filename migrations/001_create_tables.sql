-- Migration: Create Vibe MCP Server tables
-- Database: MySQL
-- Created: 2024-02-09

-- Create vibe_users table
CREATE TABLE IF NOT EXISTS vibe_users (
    id VARCHAR(36) PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    is_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_email (email)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Create vibe_tasks table
CREATE TABLE IF NOT EXISTS vibe_tasks (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    parent_task_id VARCHAR(36) NULL,
    name VARCHAR(255) NOT NULL,
    status ENUM('armed', 'running', 'completed', 'error', 'cancelled') DEFAULT 'armed',
    current_stage TEXT NULL,
    source VARCHAR(50) DEFAULT 'mcp',
    ide VARCHAR(255) NULL,
    window_title VARCHAR(500) NULL,
    project_path VARCHAR(1000) NULL,
    active_file VARCHAR(1000) NULL,
    is_focused BOOLEAN DEFAULT FALSE,
    start_time BIGINT NULL,
    end_time BIGINT NULL,
    last_heartbeat BIGINT NULL,
    estimated_duration_ms BIGINT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_user_id (user_id),
    INDEX idx_status (status),
    INDEX idx_parent_id (parent_task_id),
    
    FOREIGN KEY (user_id) REFERENCES vibe_users(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_task_id) REFERENCES vibe_tasks(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS vibe_task_stages (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    task_id VARCHAR(36) NOT NULL,
    stage TEXT NOT NULL,
    description TEXT NULL,
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

-- Create vibe_verification_codes table
CREATE TABLE IF NOT EXISTS vibe_verification_codes (
    email VARCHAR(255) PRIMARY KEY,
    code VARCHAR(10) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    INDEX idx_expires_at (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
