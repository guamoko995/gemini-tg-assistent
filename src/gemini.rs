use crate::models::ChatContext;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GeminiRequest {
    system_instruction: GeminiContent,
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
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
            model: model.unwrap_or_else(|| "gemini-3.1-flash-lite-preview".to_string()),
            name,
        }
    }

    /// Основной метод для получения ответа от ИИ.
    /// Принимает текущее саммари (сжатую память) и историю недавних сообщений.
    pub async fn generate_reply(
        &self,
        context: &ChatContext,
        chat_title: Option<&str>,
        thread_name: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        // 1. Формируем динамический системный промпт
        let system_text = get_system_prompt(&self.name, chat_title, thread_name);

        let system_instruction = GeminiContent {
            parts: vec![GeminiPart { text: system_text }],
        };

        // 2. Форматируем историю сообщений в XML
        // Используем написанный ранее форматтер format_context_to_xml
        let history_xml = format_context_to_xml(context);

        // Передаем всю историю как одно сообщение от пользователя,
        // которое я должен проанализировать и на которое должен ответить.
        let contents = vec![GeminiContent {
            parts: vec![GeminiPart {
                text: format!("Вот актуальная история переписки:\n\n{}", history_xml),
            }],
        }];

        let request = GeminiRequest {
            system_instruction,
            contents,
        };

        // 3. Отправка запроса
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

        // 4. Извлечение ответа
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

pub fn get_system_prompt(
    bot_username: &str,
    chat_title: Option<&str>,
    thread_name: Option<&str>,
) -> String {
    let location = match (chat_title, thread_name) {
        (Some(title), Some(topic)) => format!("в группе \"{}\" внутри темы \"{}\"", title, topic),
        (Some(title), None) => format!("в группе \"{}\"", title),
        (None, _) => "в личном диалоге с пользователем".to_string(),
    };

    format!(
"Ты — участник беседы {location}. Твой ник: @{bot_username}.

ПРАВИЛА ОБЩЕНИЯ В ЛЮБОМ ЧАТЕ:
1. Стиль: Лаконичный и неформальный. Пиши как в мессенджерах: коротко, по делу, без лишней вежливости и официоза.
2. Форматирование: НЕ используй Markdown (никаких звездочек и курсива). Пиши чистым текстом. Избегай сложных списков и заголовков.
3. Идентичность: Ты — нейросеть, встроенная в Telegram. Не притворяйся человеком, но и не отвечай как сухой робот-помощник. Будь органичной частью беседы.
4. Контекст: Тебе передается история сообщений в формате XML. Внимательно следи за атрибутами 'author', 'reply_to', 'quote' и 'forward_from' чтобы понимать, кто автор сообщения и к кому обращается. Твой ответ — это естественное продолжение беседы.

ФОРМАТ ИСТОРИИ:
- <summary>: краткое описание контекста диалога.
- <msg id=\"...\">: сообщение.
Атрибуты внутри тега msg могут включать:
- reply_to: id сообщения, на которое дан ответ.
- quote: текст цитируемого фрагмента, если ответ был на конкретную часть.
- forward_from: имя пользователя или чата, от которого было переслано сообщение.
Если атрибут отсутствует, значит, событие не происходило. Не пытайся искать их там, где их нет.

ПОДСКАЗКИ:
- если ты участник беседы в группе и хочешь обратиться к пользователю явно, поставь @ перед его ником, чтобы он точно не пропустил твоё сообщение.
- идентификаторы, используемые в формате истории, другим пользователям не видны, они обрабатываются клиентскими приложениями и даны тебе для удобства отслеживания ответов. В большенстве случаев не стоит о них писать.
- иногда не стоит прямо отвечать на текст пересланных сообщений, возможно они только добавляют контекста."
    )
}

pub fn format_context_to_xml(ctx: &ChatContext) -> String {
    let mut xml_ctx = String::new();

    if !ctx.summary.is_empty() {
        xml_ctx.push_str(&format!("<summary>\n{}\n</summary>\n", ctx.summary));
    }

    let mut prev_id: Option<i64> = None;

    for msg in &ctx.messages {
        let time = msg.timestamp.format("%d.%m.%Y %H:%M %Z").to_string();

        let reply_attr = match msg.reply_to_id {
            Some(id) if Some(id) != prev_id => format!(" reply_to=\"{}\"", id),
            _ => String::new(),
        };

        let sanitized_content = msg.content.replace('<', "&lt;").replace('>', "&gt;");

        let mut msg_xml = format!(
            "<msg id=\"{}\" time=\"{}\" author=\"{}\" {}>",
            msg.message_id, time, msg.user_name, reply_attr
        );

        if let Some(ref quote) = msg.quote {
            msg_xml.push_str(&format!(
                "\n<quote>{}</quote>",
                quote.replace('<', "&lt;").replace('>', "&gt;")
            ));
        }

        if let Some(ref forward_from) = msg.forward_from {
            msg_xml.push_str(&format!(
                "\n<forward_from>{}</orward_from_sender_name>",
                forward_from.replace('<', "&lt;").replace('>', "&gt;")
            ));
        }

        msg_xml.push_str(&format!("\n{}\n</msg>\n", sanitized_content));
        xml_ctx.push_str(&msg_xml);

        prev_id = Some(msg.message_id);
    }

    print!("{xml_ctx}");
    xml_ctx
}
