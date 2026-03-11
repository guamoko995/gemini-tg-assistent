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

    let bot = Bot::from_env();

    let me = bot.get_me().await?;
    let bot_username = me.user.username.as_deref().unwrap_or("bot");

    // Инициализация Gemini
    let gemini_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY missing");
    let gemini_client = Arc::new(gemini::GeminiClient::new(
        gemini_key,
        bot_username.to_string(),
        None,
    ));

    //println!("🤖 Бот запущен!");

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
