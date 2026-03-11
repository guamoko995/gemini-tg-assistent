use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Расширенное сообщение со всеми метаданными для контекста
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: i64,
    pub reply_to_id: Option<i64>,
    pub quote: Option<String>,
    pub user_id: i64,
    pub user_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Контекст, привязанный к конкретному треду
#[derive(Debug)]
pub struct ChatContext {
    pub summary: String,
    pub messages: Vec<ChatMessage>,
}
