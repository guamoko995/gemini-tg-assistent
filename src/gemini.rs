use crate::models::{ChatMessage, Role};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    role: Role,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: GeminiContent,
}

/// Клиент для взаимодействия с Google Gemini API.
#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    /// Создает новый экземпляр клиента.
    /// По умолчанию используется модель "gemini-2.5-flash".
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "gemini-2.5-flash".to_string()),
        }
    }

    /// Основной метод для получения ответа от ИИ.
    /// Принимает текущее саммари (сжатую память) и историю недавних сообщений.
    pub async fn ask(
        &self,
        summary: &str,
        history: &[ChatMessage],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let mut contents = Vec::new();

        // 1. Если есть саммари, добавляем его как вводный контекст от пользователя
        if !summary.is_empty() {
            contents.push(GeminiContent {
                role: Role::User,
                parts: vec![GeminiPart {
                    text: format!("Краткое содержание предыдущей части беседы: {}. Учти это и продолжай диалог.", summary),
                }],
            });
            // Добавляем "подтверждение" от модели, чтобы соблюсти чередование User/Model
            contents.push(GeminiContent {
                role: Role::Model,
                parts: vec![GeminiPart {
                    text: "Понял. Я помню контекст нашего разговора.".to_string(),
                }],
            });
        }

        // 2. Переносим историю сообщений из базы данных
        for msg in history {
            contents.push(GeminiContent {
                role: msg.role,
                parts: vec![GeminiPart {
                    text: msg.content.clone(),
                }],
            });
        }

        let request = GeminiRequest { contents };

        let response: GeminiResponse = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        // Извлекаем текст из первого кандидата, проверяя наличие данных
        let text = response
            .candidates
            .as_ref()
            .and_then(|c| c.get(0))
            .map(|c| &c.content.parts)
            .and_then(|p| p.get(0))
            .map(|p| p.text.clone())
            .ok_or("Не удалось получить текстовый ответ от Gemini API")?;

        Ok(text)
    }
}
