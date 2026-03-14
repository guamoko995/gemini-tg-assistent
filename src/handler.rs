use crate::db;
use crate::gemini::GeminiClient;
use crate::models::{ChatContext, ChatMessage};

use sqlx::SqlitePool;
use std::sync::Arc;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{MessageOrigin, ReplyParameters};

/// Основной обработчик входящих сообщений
pub async fn message_handler(
    bot: Bot,
    msg: Message,
    pool: Arc<SqlitePool>,
    gemini: Arc<GeminiClient>,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let thread_id = match msg.thread_id {
        Some(thread_id) => thread_id.0.0,
        None => 0,
    } as i64;
    let bot_user = bot.get_me().await?;
    let bot_username = bot_user.user.username.as_deref().unwrap_or("bot");

    let chat_title = msg.chat.title();
    // Имя темы вытащить сложнее, пока оставим None или поищем в msg.thread_id
    let thread_name: Option<&str> = None;

    // 1. Извлекаем текст
    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()), // Игнорируем стикеры/фото для минимализма
    };

    // 2. Сохраняем входящее сообщение в базу (всегда, для контекста)
    let count = match db::upsert_message(&*pool, chat_id, thread_id, &to_chat_message(&msg)).await {
        Ok(count) => count,
        Err(err) => {
            log::error!("Ошибка сохранения сообщения: {:?}", err);
            0
        }
    };

    // 3. Проверяем, нужно ли отвечать (тег бота или ответ на его сообщение)
    let is_mentioned = text.contains(&format!("@{}", bot_username));
    let is_reply_to_bot = msg
        .reply_to_message()
        .and_then(|m| m.from.clone())
        .map(|u| u.id == bot_user.user.id)
        .unwrap_or(false);

    let need_response = is_mentioned || is_reply_to_bot || msg.chat.is_private();
    let need_sammory = count >= 100;

    if need_response || need_sammory {
        // 4. Получаем контекст из БД
        let context = match db::get_chat_context(&*pool, chat_id, thread_id).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Ошибка получения контекста: {:?}", e);
                return Ok(());
            }
        };

        if need_response {
            // Показываем статус "печатает"
            bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                .await?;

            // 5. Запрос к Gemini
            match gemini
                .generate_reply(&context, chat_title, thread_name)
                .await
            {
                Ok(ai_response) => {
                    // 6. Отправляем ответ пользователю
                    let resp_msg = send_bot_reply(&bot, &msg, &ai_response).await?;

                    // 7. Сохраняем ответ бота в базу
                    let _ =
                        db::upsert_message(&*pool, chat_id, thread_id, &to_chat_message(&resp_msg))
                            .await;
                }
                Err(e) => {
                    log::error!("Ошибка Gemini: {:?}", e);
                    bot.send_message(msg.chat.id, "Извини, я временно не могу сообразить...")
                        .await?;
                }
            }
        }

        if need_sammory {
            // 8. Архивируем контекст
            if let Err(e) =
                check_and_summarize(pool.clone(), gemini.clone(), chat_id, thread_id, &context)
                    .await
            {
                log::error!("Ошибка фоновой архивации: {:?}", e);
            }
        }
    }

    Ok(())
}

pub async fn send_bot_reply(
    bot: &Bot,
    original_msg: &Message,
    text: &str,
) -> ResponseResult<Message> {
    let mut resp = bot.send_message(original_msg.chat.id, text);

    if !original_msg.chat.is_private() {
        resp = resp.reply_parameters(ReplyParameters::new(original_msg.id));
    }

    if let Some(thread_id) = original_msg.thread_id {
        resp = resp.message_thread_id(thread_id);
    }

    resp.await
}

fn to_chat_message(msg: &Message) -> ChatMessage {
    ChatMessage {
        message_id: msg.id.0 as i64,
        reply_to_id: msg.reply_to_message().map(|m| m.id.0 as i64),
        user_id: msg.from.clone().map(|u| u.id.0 as i64).unwrap_or(0),
        user_name: msg
            .from
            .as_ref()
            .and_then(|u| u.username.clone())
            .unwrap_or("bot".to_string()),
        content: msg.text().unwrap().to_string(),
        quote: msg.quote().map(|q| q.text.to_string()),
        forward_from: get_forward_name(msg.forward_origin()),
        timestamp: chrono::Utc::now(),
    }
}

pub fn get_forward_name(origin: Option<&MessageOrigin>) -> Option<String> {
    origin.map(|o| match o {
        MessageOrigin::User { sender_user, .. } => sender_user.first_name.clone(),
        MessageOrigin::HiddenUser {
            sender_user_name, ..
        } => sender_user_name.clone(),
        MessageOrigin::Chat { sender_chat, .. } => {
            sender_chat.title().unwrap_or("unknown").to_string()
        }
        MessageOrigin::Channel { chat, .. } => chat.title().unwrap_or("unknown").to_string(),
    })
}

async fn check_and_summarize(
    pool: Arc<SqlitePool>,
    gemini: Arc<GeminiClient>,
    chat_id: i64,
    thread_id: i64,
    context: &ChatContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if context.messages.len() <= 100 {
        return Ok(());
    }

    match gemini.generate_summary(context).await {
        Ok((new_summary, last_id)) => {
            db::archive_thread_messages(&*pool, chat_id, thread_id, &new_summary, last_id).await?;
        }
        Err(e) => log::error!("Ошибка суммаризации: {:?}", e),
    }

    Ok(())
}
