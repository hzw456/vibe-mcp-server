-- Vibe MCP Server Database Migration
-- MySQL Database Schema for Persistent Task Storage

-- Create database if not exists
CREATE DATABASE IF NOT EXISTS vibe_db CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

USE vibe_db;

-- Drop existing tables (for clean reinstall)
DROP TABLE IF EXISTS vibe_verification_codes;
DROP TABLE IF EXISTS vibe_tasks;
DROP TABLE IF EXISTS vibe_users;

-- Create users table
CREATE TABLE vibe_users (
    id VARCHAR(36) PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    is_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_email (email)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Create tasks table
CREATE TABLE vibe_tasks (
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

-- Create verification codes table
CREATE TABLE vibe_verification_codes (
    email VARCHAR(255) PRIMARY KEY,
    code VARCHAR(10) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_expires_at (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Insert sample test user (password: testpass123)
-- Note: This is a sample user for testing purposes
INSERT INTO vibe_users (id, email, password_hash, is_verified) VALUES 
('test-user-001', 'testuser@vibe.app', '$2b$06$test_hash_placeholder', TRUE);

-- Comments:
-- - All tables use utf8mb4 for full Unicode support including emojis
-- - Foreign keys ensure data integrity between users and tasks
-- - Indexes on user_id, status, and parent_task_id for efficient queries
-- - TIMESTAMP fields use DEFAULT CURRENT_TIMESTAMP for automatic timestamps
-- - ON UPDATE CURRENT_TIMESTAMP for updated_at to auto-update on modification
