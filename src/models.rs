use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};

/// Расширенное сообщение со всеми метаданными для контекста
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessage {
    pub tg_message_id: i64,
    pub reply_to_id: Option<i64>,
    pub user_id: i64,
    pub user_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Контекст, привязанный к конкретному треду
#[derive(Debug)]
pub struct ChatContext {
    pub chat_id: i64,
    pub thread_id: i64,
    pub summary: String,
    pub messages: Vec<ChatMessage>,
}
