use mysql::Pool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = "mysql://root:TqB_A9ywSJv5PKk@172.21.16.12:3306/vibe_db";

    println!("Connecting to database: {}", database_url);

    let pool = Pool::new(database_url).unwrap();
    println!("✅ Database connection successful!");

    // Test query
    let result = pool.prep_exec("SELECT 1", ()).await?;
    println!("Test query result: {:?}", result);

    println!("✅ All tests passed!");

    Ok(())
}
