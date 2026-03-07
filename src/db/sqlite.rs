use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::env;

pub async fn init_db() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool: SqlitePool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await?;

    sqlx::query(include_str!("schema.sql"))
        .execute(&pool)
        .await?;

    Ok(pool)
}
