use sqlx::PgPool;
use eyre::Result;

pub mod transfers;

pub async fn init_db(database_url: &str) -> Result<PgPool> {
    // Create the connection pool
    let pool = PgPool::connect(database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./src/migrations")
        .run(&pool)
        .await?;

    println!("Database initialized successfully");
    Ok(pool)
} 