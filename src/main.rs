mod db;
mod gemini;
mod handler;
mod models;

use dotenvy::dotenv;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;
use teloxide::prelude::*;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    pretty_env_logger::init(); // Для логов в терминале

    // Инициализация БД
    let pool: Arc<SqlitePool> = Arc::new(db::init_db().await?);

    // Инициализация Gemini
    let gemini_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY missing");
    let gemini_client = Arc::new(gemini::GeminiClient::new(gemini_key, None));

    let bot = Bot::from_env();

    println!("🤖 Бот запущен!");

    // Настройка Dispatcher
    let handler = Update::filter_message().endpoint(handler::message_handler);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![pool, gemini_client])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
