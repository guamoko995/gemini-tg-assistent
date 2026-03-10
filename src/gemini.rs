use crate::models::{ChatMessage, Role};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GeminiRequest {
    system_instruction: GeminiContent,
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: Option<Role>,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: GeminiContent,
}

/// Клиент для взаимодействия с Google Gemini API.
#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    name: String,
}

impl GeminiClient {
    /// Создает новый экземпляр клиента.
    /// По умолчанию используется модель "gemini-2.5-flash".
    pub fn new(api_key: String, name: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "gemini-3-flash-preview".to_string()),
            name,
        }
    }

    /// Основной метод для получения ответа от ИИ.
    /// Принимает текущее саммари (сжатую память) и историю недавних сообщений.
    pub async fn chat(
        &self,
        summary: &str,
        history: &[ChatMessage],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let system_instruction = GeminiContent {
            role: None,
            parts: vec![GeminiPart {
                text: format!(include_str!("gemini/chatter-prompt.md"), self.name),
            }],
        };

        let mut contents: Vec<GeminiContent> = Vec::new();

        // 1. Если есть саммари, добавляем его как вводный контекст от пользователя
        if !summary.is_empty() {
            contents.push(GeminiContent {
                role: Some(Role::User),
                parts: vec![GeminiPart {
                    text: format!("Краткое содержание предыдущей части беседы:\n\n {}\n\n Учти это и продолжай диалог.", summary),
                }],
            });

            // Добавляем "подтверждение" от модели, чтобы соблюсти чередование User/Model
            contents.push(GeminiContent {
                role: Some(Role::Model),
                parts: vec![GeminiPart {
                    text: "Да, я помню контекст нашего разговора.".to_string(),
                }],
            });
        }

        // 2. Переносим историю сообщений из базы данных
        for msg in history {
            contents.push(GeminiContent {
                role: Some(msg.role),
                parts: vec![GeminiPart {
                    text: format!("{}: {}", msg.user.clone(), msg.content.clone()),
                }],
            });
        }

        let request = GeminiRequest {
            system_instruction,
            contents,
        };

        println!("{request:?}");

        let response: GeminiResponse = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        println!("{response:?}");

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
