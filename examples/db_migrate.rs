use mysql::{prelude::Queryable, Pool};
use std::process;

fn main() {
    let database_url = "mysql://root:TqB_A9ywSJv5PKk@172.21.16.12:3306/vibe_db";

    println!("Connecting to database: {}", database_url);

    let pool = match Pool::new(database_url) {
        Ok(pool) => {
            println!("✅ Database connection successful!");
            pool
        }
        Err(e) => {
            eprintln!("❌ Failed to connect to database: {}", e);
            process::exit(1);
        }
    };

    // Get a connection
    let mut conn = match pool.get_conn() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("❌ Failed to get connection: {}", e);
            process::exit(1);
        }
    };

    // Create vibe_users table
    println!("Creating vibe_users table...");
    if let Err(e) = conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS vibe_users (
            id VARCHAR(36) PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            is_verified BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            INDEX idx_email (email)
        )
    "#,
    ) {
        eprintln!("❌ Failed to create vibe_users table: {}", e);
        process::exit(1);
    }
    println!("✅ vibe_users table created successfully!");

    // Create vibe_tasks table
    println!("Creating vibe_tasks table...");
    if let Err(e) = conn.query_drop(
        r#"
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
        )
    "#,
    ) {
        eprintln!("❌ Failed to create vibe_tasks table: {}", e);
        process::exit(1);
    }
    println!("✅ vibe_tasks table created successfully!");

    // Create vibe_verification_codes table
    println!("Creating vibe_verification_codes table...");
    if let Err(e) = conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS vibe_verification_codes (
            email VARCHAR(255) PRIMARY KEY,
            code VARCHAR(10) NOT NULL,
            expires_at TIMESTAMP NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            INDEX idx_expires_at (expires_at)
        )
    "#,
    ) {
        eprintln!("❌ Failed to create vibe_verification_codes table: {}", e);
        process::exit(1);
    }
    println!("✅ vibe_verification_codes table created successfully!");

    println!("\n🎉 All tables created successfully!");
    println!("Database migration completed!");
}
