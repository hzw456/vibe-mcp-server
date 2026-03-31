//! Vibe MCP Server - AI Task Status Tracker
//! 基于 MCP 协议的 AI 任务状态跟踪服务

use vibe_mcp_server::{create_router, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = dotenvy::dotenv();
    env_logger::init();

    let config = Config::from_args(std::env::args())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    let state = vibe_mcp_server::AppState::new(config.clone()).await?;
    let router = create_router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("🚀 Vibe MCP Server started on {}", addr);
    println!("📡 Health: http://{}:{}/health", addr, config.port);
    println!("🔐 Auth:   http://{}:{}/api/auth/login", addr, config.port);
    println!("📋 Tasks:  http://{}:{}/api/status", addr, config.port);
    println!("🔌 MCP:    http://{}:{}/mcp", addr, config.port);
    if config.database_url.is_empty() {
        println!("💾 Database: Not configured");
    } else {
        println!("💾 Database: Configured");
    }
    println!("\n📝 Environment variables:");
    println!("   API_KEY={}", config.api_key);
    println!("   JWT_SECRET={}", config.jwt_secret);
    println!("   JWT_EXPIRY_HOURS={}", config.jwt_expiry_hours);

    axum::serve(listener, router).await?;
    Ok(())
}
