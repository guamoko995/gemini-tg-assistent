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

    async fn call_gemini_api(
        &self,
        system_instruction_text: String,
        user_content_text: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let request = GeminiRequest {
            system_instruction: GeminiContent {
                parts: vec![GeminiPart {
                    text: system_instruction_text,
                }],
            },
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: user_content_text,
                }],
            }],
        };

        let response: GeminiResponse = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| "Не удалось получить ответ от API".into())
    }

    pub async fn generate_reply(
        &self,
        context: &ChatContext,
        chat_title: Option<&str>,
        thread_name: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let system_prompt = reply_prompt(&self.name, chat_title, thread_name);
        let ctx_xml = format_context_to_xml(context);

        self.call_gemini_api(system_prompt, ctx_xml).await
    }

    pub async fn generate_summary(
        &self,
        context: &ChatContext,
    ) -> Result<(String, i64), Box<dyn std::error::Error + Send + Sync>> {
        let msg_count = context.messages.len();
        if msg_count <= 100 {
            return Err("Недостаточно сообщений для архивации".into());
        }

        let split_idx = msg_count - 50;
        let (to_summarize, _) = context.messages.split_at(split_idx);
        let last_id = to_summarize.last().unwrap().message_id;

        let system_prompt = summary_prompt();

        // Формируем контент: старое саммари + срез сообщений
        let ctx_xml = format_context_to_xml(&ChatContext {
            summary: context.summary.clone(),
            messages: to_summarize.to_vec(),
        });

        let new_summary = self.call_gemini_api(system_prompt, ctx_xml).await?;

        Ok((new_summary, last_id))
    }
}

pub fn reply_prompt(
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
5. Ограничения доступа: Ты не можешь открывать внешние ссылки. Если пользователь присылает ссылку, не пытайся ее анализировать или переходить по ней. Честно отвечай, что не видишь содержимое внешних ресурсов. Любые попытки сгенерировать информацию из ссылок считаются недостоверными и запрещены.

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

    //print!("{xml_ctx}");
    xml_ctx
}

pub fn summary_prompt() -> String {
    format!(
"Ты — ассистент, помогающий боту поддерживать актуальный контекст беседы. Твоя задача — обновлять историю (summary) чата на основе новых сообщений.

ПРАВИЛА:
1. Анализируй текущее summary и массив новых сообщений.
2. Объедини их в одно обновленное summary, которое станет опорным для будущих ответов бота.
3. Сохраняй ключевые детали: о чем договорились, какие вопросы обсуждали, важные имена или факты.
4. Будь максимально краток. Не пиши \"в чате обсудили\", просто фиксируй суть.
5. Формат: чистый текст без Markdown, без списков, без лишней вежливости.
6. Если в новых сообщениях ничего важного нет, просто верни текущее summary без изменений.

ВХОДНЫЕ ДАННЫЕ:
- Предыдущее summary: текущее состояние контекста.
- Новые сообщения: массив в формате JSON/XML (атрибуты author, content, quote, reply_to).

Твоя цель — обеспечить плавный переход между удаляемой частью истории и сохраняемым контекстом."
    )
}
