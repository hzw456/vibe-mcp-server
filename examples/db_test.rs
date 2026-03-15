use mysql::{prelude::Queryable, Pool};
use std::process;
use std::time::Duration;

fn main() {
    let database_url = "mysql://root:TqB_A9ywSJv5PKk@172.21.16.12:3306/vibe_db";

    println!("Connecting to database: {}", database_url);

    let opts = mysql::OptsBuilder::new()
        .ip_or_hostname(Some("172.21.16.12"))
        .tcp_port(3306)
        .user(Some("root"))
        .pass(Some("TqB_A9ywSJv5PKk"))
        .db_name(Some("vibe_db"));

    let pool = match Pool::new(opts) {
        Ok(pool) => {
            println!("✅ Pool created successfully!");
            pool
        }
        Err(e) => {
            eprintln!("❌ Failed to create pool: {}", e);
            process::exit(1);
        }
    };

    println!("Testing connection...");

    // Get a connection with timeout
    let mut conn = match pool.get_conn() {
        Ok(conn) => {
            println!("✅ Connection established!");
            conn
        }
        Err(e) => {
            eprintln!("❌ Failed to get connection: {}", e);
            process::exit(1);
        }
    };

    // Test query
    println!("Running test query...");
    let result: Vec<mysql::Row> = conn.query("SELECT 1").unwrap();
    println!("✅ Test query result: {:?}", result);

    println!("\n🎉 Database connection successful!");
}
