use crate::db;
use crate::gemini::GeminiClient;
use crate::models::ChatMessage;

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
    if let Err(e) = db::upsert_message(&*pool, chat_id, thread_id, &to_chat_message(&msg)).await {
        log::error!("Ошибка сохранения сообщения: {:?}", e);
    }

    // 3. Проверяем, нужно ли отвечать (тег бота или ответ на его сообщение)
    let is_mentioned = text.contains(&format!("@{}", bot_username));
    let is_reply_to_bot = msg
        .reply_to_message()
        .and_then(|m| m.from.clone())
        .map(|u| u.id == bot_user.user.id)
        .unwrap_or(false);

    if is_mentioned || is_reply_to_bot || msg.chat.is_private() {
        // Показываем статус "печатает"
        bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
            .await?;

        // 4. Получаем контекст из БД
        let context = match db::get_chat_context(&*pool, chat_id, thread_id).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Ошибка получения контекста: {:?}", e);
                return Ok(());
            }
        };

        // 5. Запрос к Gemini
        match gemini
            .generate_reply(&context, chat_title, thread_name)
            .await
        {
            Ok(ai_response) => {
                // 6. Отправляем ответ пользователю
                let mut resp_msg = bot.send_message(msg.chat.id, &ai_response);

                if !msg.chat.is_private() {
                    resp_msg = resp_msg.reply_parameters(ReplyParameters::new(msg.id))
                };

                if let Some(thread_id) = msg.thread_id {
                    resp_msg = resp_msg.message_thread_id(thread_id)
                };

                let resp_msg = resp_msg.await?;

                // 7. Сохраняем ответ бота в базу
                let _ = db::upsert_message(&*pool, chat_id, thread_id, &to_chat_message(&resp_msg))
                    .await;
            }
            Err(e) => {
                log::error!("Ошибка Gemini: {:?}", e);
                bot.send_message(msg.chat.id, "Извини, я временно не могу сообразить...")
                    .await?;
            }
        }
    }

    Ok(())
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
