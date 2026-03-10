use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};

/// Роль участника диалога, совместимая с Gemini API.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Model,
}

/// Сообщение из истории чата.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub user: String,
    pub content: String,
}

/// Контекст для передачи в ИИ: накопленный итог + свежие сообщения.
#[derive(Debug)]
pub struct ChatContext {
    pub summary: String,
    pub messages: Vec<ChatMessage>,
}
